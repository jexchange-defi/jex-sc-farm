#![no_std]

multiversx_sc::imports!();

const SAFETY_CONSTANT: u64 = 1_000_000_000_000_000_000u64;

#[multiversx_sc::contract]
pub trait FarmContract {
    /// Initialize the smart contract
    /// Note that staking_token and rewards_token can be equals
    #[init]
    fn init(&self, staking_token: TokenIdentifier, rewards_token: TokenIdentifier) {
        self.staking_token().set_if_empty(&staking_token);
        self.rewards_token().set_if_empty(&rewards_token);
    }

    // Owner Endpoints

    /// Fund and start rewards distribution
    /// Important: make sure the payment amount can be divided by the rewards duration.
    /// Eg: 1000,000000 as rewards for 3600 seconds -> fail
    /// Eg: 1000,000000 as rewards for 4000 seconds -> OK (0,250000 per second)
    #[endpoint]
    #[only_owner]
    #[payable("*")]
    fn fund(&self) {
        let payment = self.call_value().single_esdt();

        require!(
            payment.token_identifier == self.rewards_token().get(),
            "Wrong rewards token"
        );

        let block_ts = self.blockchain().get_block_timestamp();
        if block_ts >= self.finish_at().get() {
            self.reward_per_second()
                .set(payment.amount / self.rewards_duration().get());
        } else {
            let leftover = self
                .reward_per_second()
                .get()
                .mul(self.finish_at().get() - block_ts);
            self.reward_per_second()
                .set((payment.amount + leftover) / self.rewards_duration().get());
        }

        let balance = self.blockchain().get_sc_balance(
            &EgldOrEsdtTokenIdentifier::esdt(self.rewards_token().get()),
            0,
        );

        let finish_at = block_ts + self.rewards_duration().get();
        self.finish_at().set(finish_at);

        require!(
            balance == self.reward_per_second().get() * (finish_at - block_ts),
            "Invalid rewards balance"
        );
    }

    /// Set rewards distribution duration (in seconds)
    /// Can only be called when distribution has not started (or previous one is complete)
    #[endpoint(setRewardsDuration)]
    #[only_owner]
    fn set_rewards_duration(&self, duration: u64) {
        require!(self.blockchain().get_block_timestamp() > self.finish_at().get(),
            "Previous rewards period must be complete before changing the duration for the new period");

        self.rewards_duration().set(duration);
    }

    // Public Endpoints

    /// Claim rewards
    #[endpoint]
    fn claim(&self) {
        let caller = self.blockchain().get_caller();
        self.update_reward(&caller);

        let reward = self.rewards(&caller).get();

        if reward > 0 {
            self.rewards(&caller).clear();

            self.send()
                .direct_esdt(&caller, &self.rewards_token().get(), 0, &reward);
        }
    }

    /// Exit (withdraw+claim)
    #[endpoint]
    fn exit(&self) {
        let caller = self.blockchain().get_caller();

        self.withdraw(self.balance_of(&caller).get());

        self.claim();

        self.all_stakers().swap_remove(&caller);
    }

    /// Add tokens to staking
    #[endpoint]
    #[payable("*")]
    fn stake(&self) {
        let payment = self.call_value().single_esdt();
        require!(
            payment.token_identifier == self.staking_token().get(),
            "Wrong staking token"
        );

        let caller = self.blockchain().get_caller();
        self.update_reward(&caller);

        self.balance_of(&caller).update(|x| *x += &payment.amount);
        self.total_staked().update(|x| *x += &payment.amount);

        self.all_stakers().insert(caller);
    }

    /// Withdraw tokens from staking
    /// This endpoint does not claim the rewards
    #[endpoint]
    fn withdraw(&self, amount: BigUint) {
        let caller = self.blockchain().get_caller();

        require!(amount <= self.balance_of(&caller).get(), "Invalid amount");

        self.update_reward(&caller);

        self.total_staked().update(|x| *x -= &amount);
        self.balance_of(&caller).update(|x| *x -= &amount);

        if self.balance_of(&caller).is_empty() {
            self.all_stakers().swap_remove(&caller);
        }

        self.send()
            .direct_esdt(&caller, &self.staking_token().get(), 0, &amount);
    }

    // Functions

    fn compute_reward_per_token(&self) -> BigUint {
        if self.total_staked().is_empty() {
            return self.reward_per_token().get();
        }

        let rpt = self.reward_per_token().get().add(
            self.reward_per_second()
                .get()
                .mul(self.last_time_reward_applicable() - self.updated_at().get())
                .mul(SAFETY_CONSTANT)
                / self.total_staked().get(),
        );
        rpt
    }

    fn earned(&self, account: &ManagedAddress) -> BigUint {
        let earned = self
            .balance_of(&account)
            .get()
            .mul(self.compute_reward_per_token() - self.user_reward_per_token_paid(account).get())
            .div(SAFETY_CONSTANT)
            .add(self.rewards(account).get());
        earned
    }

    fn last_time_reward_applicable(&self) -> u64 {
        let ts = u64::min(
            self.finish_at().get(),
            self.blockchain().get_block_timestamp(),
        );
        ts
    }

    fn update_reward(&self, account: &ManagedAddress) {
        self.reward_per_token().set(self.compute_reward_per_token());
        self.updated_at().set(self.last_time_reward_applicable());

        self.rewards(&account).set(self.earned(&account));
        self.user_reward_per_token_paid(&account)
            .set(self.reward_per_token().get());
    }

    // Storage & Views

    #[view(getAllStakers)]
    fn get_all_stakers(&self, from: usize, size: usize) -> ManagedVec<Self::Api, ManagedAddress> {
        let stakers: ManagedVec<Self::Api, ManagedAddress> =
            self.all_stakers().iter().skip(from).take(size).collect();
        stakers
    }

    #[view(getPendingRewards)]
    fn get_pending_rewards(&self, account: &ManagedAddress) -> BigUint {
        return self.earned(account);
    }

    #[storage_mapper("all_stakers")]
    fn all_stakers(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[view(getBalanceOf)]
    #[storage_mapper("balance_of")]
    fn balance_of(&self, account: &ManagedAddress) -> SingleValueMapper<BigUint>;

    #[view(getFinishAt)]
    #[storage_mapper("finish_at")]
    fn finish_at(&self) -> SingleValueMapper<u64>;

    #[view(getRewards)]
    #[storage_mapper("rewards")]
    fn rewards(&self, account: &ManagedAddress) -> SingleValueMapper<BigUint>;

    #[view(getRewardsDuration)]
    #[storage_mapper("rewards_duration")]
    fn rewards_duration(&self) -> SingleValueMapper<u64>;

    #[view(getRewardPerSecond)]
    #[storage_mapper("reward_per_second")]
    fn reward_per_second(&self) -> SingleValueMapper<BigUint>;

    #[view(getRewardPerToken)]
    #[storage_mapper("reward_per_token")]
    fn reward_per_token(&self) -> SingleValueMapper<BigUint>;

    #[view(getRewardsToken)]
    #[storage_mapper("rewards_token")]
    fn rewards_token(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getStakingToken)]
    #[storage_mapper("staking_token")]
    fn staking_token(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getTotalStaked)]
    #[storage_mapper("total_staked")]
    fn total_staked(&self) -> SingleValueMapper<BigUint>;

    #[view(getUpdatedAt)]
    #[storage_mapper("updated_at")]
    fn updated_at(&self) -> SingleValueMapper<u64>;

    #[view(getUserRewardPerTokenPaid)]
    #[storage_mapper("user_reward_per_token_paid")]
    fn user_reward_per_token_paid(&self, account: &ManagedAddress) -> SingleValueMapper<BigUint>;
}
