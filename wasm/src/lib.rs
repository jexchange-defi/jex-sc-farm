// Code generated by the multiversx-sc build system. DO NOT EDIT.

////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

// Init:                                 1
// Endpoints:                           22
// Async Callback (empty):               1
// Total number of exported functions:  24

#![no_std]
#![allow(internal_features)]
#![feature(lang_items)]

multiversx_sc_wasm_adapter::allocator!();
multiversx_sc_wasm_adapter::panic_handler!();

multiversx_sc_wasm_adapter::endpoints! {
    jex_sc_farm
    (
        init => init
        upgrade => upgrade
        fund => fund
        setRewardsDuration => set_rewards_duration
        terminate => terminate
        claim => claim
        exit => exit
        stake => stake
        withdraw => withdraw
        getStatus => get_status
        getAllStakers => get_all_stakers
        getPendingRewards => get_pending_rewards
        getBalanceOf => balance_of
        getFinishAt => finish_at
        getRewards => rewards
        getRewardsDuration => rewards_duration
        getRewardPerSecond => reward_per_second
        getRewardPerToken => reward_per_token
        getRewardsToken => rewards_token
        getStakingToken => staking_token
        getTotalStaked => total_staked
        getUpdatedAt => updated_at
        getUserRewardPerTokenPaid => user_reward_per_token_paid
    )
}

multiversx_sc_wasm_adapter::async_callback_empty! {}
