use crate::certora_specs::mocks::conversions::certora_convert_to_shares;
use crate::certora_specs::mocks::conversions::certora_convert_to_tokens;
use crate::PoolBalance;
use crate::backstop::require_is_from_pool_factory;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Env, Address};

use cvlr::asserts::{cvlr_assert, cvlr_assume};
#[cfg(feature = "certora")]
use crate::certora_specs::{self, mocks::{self, pool_factory::*}};
#[cfg(feature = "certora")]
use crate::certora_specs::mocks::pool_factory::GHOST_IS_POOL;

// converting 0 units of tokens/shares will lead to 0 units
#[rule]
pub fn conversion_of_zero(pool_balance: &mut PoolBalance) {
    let tokens = pool_balance.convert_to_tokens(0);
    let shares = pool_balance.convert_to_shares(0);
    cvlr_assert!(tokens == 0 && shares == 0);
}

// token -> shares gives tokens when pool has 0 shares
#[rule]
pub fn conversion_pool_zero_shares(pool_balance: &mut PoolBalance, tokens: i128) {
    cvlr_assume!(pool_balance.shares == 0); //, "pool has 0 shares");
    let shares = pool_balance.convert_to_shares(tokens);
    cvlr_assert!(shares == tokens);
    cvlr_assert!(pool_balance.shares == 0);
}

// simpler correctness of token conversion for i64 variant
#[rule]
pub fn simple_token_roundtrip_correct(pool_shares: i64, pool_tokens: i64, tokens: i64) {
    cvlr_assume!(
        tokens >= 0 && pool_shares > 0 && pool_tokens > 0);
    let tokens_res = certora_convert_to_tokens(
        pool_shares,
        pool_tokens,
        certora_convert_to_shares(pool_shares, pool_tokens, tokens),
    );
    cvlr_assert!(tokens >= tokens_res);
}

// simpler correctness of share conversion for i64 variant
#[rule]
pub fn simple_share_roundtrip_correct(pool_shares: i64, pool_tokens: i64, shares: i64) {
    cvlr_assume!(
        shares >= 0 && pool_shares > 0 && pool_tokens > 0); 
    let shares_res = certora_convert_to_shares(
        pool_shares,
        pool_tokens,
        certora_convert_to_tokens(pool_shares, pool_tokens, shares),
    );
    cvlr_assert!(shares >= shares_res);
}

// RULES FOR PRIVATE MUTANTS

// require_is_from_pool_factory should fail if its address wasn't deployed by the Pool Factory
#[rule]
pub fn require_is_from_pool_factory_correct(e: &Env, address: &Address, balance: i128) {
    unsafe {
        GHOST_IS_POOL.init(false);
    }

    cvlr_assume!(balance == 0);
    require_is_from_pool_factory(e, address, balance);
    
    unsafe {
        let is_pool = GHOST_IS_POOL.get();
        cvlr_assert!(is_pool);
    }
}

// PoolBalance RULES
// 1. CORRECTNESS RULES

// deposit correctness
#[rule]
pub fn deposit_correct(pool_balance: &mut PoolBalance, amount: i128, shares: i128) {
    let tokens_before = pool_balance.tokens;
    let shares_before = pool_balance.shares;
    pool_balance.deposit(amount, shares);
    let tokens_after = pool_balance.tokens;
    let shares_after = pool_balance.shares;
    cvlr_assert!((tokens_after == tokens_before + amount) &&
        (shares_after == shares_before + shares));
}

// RULE FOR PUBLIC MUTANTS
// withdraw correctness
#[rule]
pub fn withdraw_correct(pool_balance: &mut PoolBalance, e: &Env, amount: i128, shares: i128) {
    let tokens_before = pool_balance.tokens;
    let shares_before = pool_balance.shares;
    let q4w_before = pool_balance.q4w; 
    pool_balance.withdraw(e, amount, shares);
    let tokens_after = pool_balance.tokens;
    let shares_after = pool_balance.shares;
    let q4w_after = pool_balance.q4w;
    cvlr_assert!((tokens_after == tokens_before - amount) &&
        (shares_after == shares_before - shares) &&
        (q4w_after == q4w_before - shares));
}

// queue_for_withdraw correctness
#[rule]
pub fn queue_for_withdraw_correct(pool_balance: &mut PoolBalance, shares: i128) {
    let q4w_before = pool_balance.q4w; 
    pool_balance.queue_for_withdraw(shares);
    let q4w_after = pool_balance.q4w;
    cvlr_assert!(q4w_after == q4w_before + shares);
}

// dequeue_q4w correctness
#[rule]
pub fn dequeue_q4w_correct(pool_balance: &mut PoolBalance, e: &Env, shares: i128) {
    let q4w_before = pool_balance.q4w;
    pool_balance.dequeue_q4w(e, shares);
    let q4w_after = pool_balance.q4w;
    cvlr_assert!(q4w_after == q4w_before - shares);
}

// 2. ASSERTIONS CHECKS

// withdraw should fail if requested amount of tokens/shares > available amount
#[rule]
pub fn withdraw_insufficient_funds_fails(e: &Env, amount: i128, shares: i128) {
    let mut pool_balance: PoolBalance = cvlr::nondet();
    cvlr_assume!((amount > pool_balance.tokens) || (shares > pool_balance.shares) || (shares > pool_balance.q4w));
    pool_balance.withdraw(e, amount, shares);
    cvlr_assert!(false); //should pass
}

// dequeue_q4w should fail if requested amount of shares > amount in q4w
#[rule]
pub fn dequeue_q4w_insufficient_funds_fails(pool_balance: &mut PoolBalance, e: &Env, shares: i128) {
    cvlr_assume!(shares > pool_balance.q4w);
    pool_balance.dequeue_q4w(e, shares);
    cvlr_assert!(false);
}
