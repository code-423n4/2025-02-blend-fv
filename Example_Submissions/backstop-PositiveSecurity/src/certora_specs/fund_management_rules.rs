use cvlr::cvlr_assert;
use cvlr::cvlr_assume;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::{backstop::{execute_draw, execute_donate}, storage};

// ghosts for verification purposes
#[cfg(feature = "certora")]
use crate::certora_specs::mocks::pool_factory::GHOST_IS_POOL;

// EXECUTE_DRAW RULES
// 1. CORRECTNESS RULES

// execute_draw correctly updates pool balance
#[rule]
pub fn execute_draw_pb_update_correct(e: &Env, pool_address: &Address, amount: i128, to: &Address) {
    let pb_before = storage::get_pool_balance(e, pool_address);
    cvlr_assume!(amount >= 0 && amount <= pb_before.tokens);
    execute_draw(e, pool_address, amount, to);
    let pb_after = storage::get_pool_balance(e, pool_address);
    cvlr_assert!(
        (pb_after.tokens == pb_before.tokens - amount) &&
        (pb_after.shares == pb_before.shares) &&
        (pb_after.q4w == pb_before.q4w)
    );
}

// 2. ASSERTIONS CHECKS 

// execute_draw should fail if amount < 0
#[rule]
pub fn draw_negative_amount_fails(e: &Env, pool: &Address, to: &Address, amount: i128) {
    cvlr_assume!(amount < 0);
    execute_draw(e, pool, amount, to);
    cvlr_assert!(false);
}

// execute_draw should fail if requested amount is greater than pool_balance
#[rule]
pub fn draw_pb_insuficient_balance_fails(e: &Env, pool_address: &Address, amount: i128, to: &Address) {
    let pb_before = storage::get_pool_balance(e, pool_address);
    cvlr_assume!(amount > pb_before.tokens);
    execute_draw(e, pool_address, amount, to);
    cvlr_assert!(false);
}

// EXECUTE_DONATE RULES
// 1. CORRECTNESS RULES

// execute_donate adds sent amount of tokens to pool but no shares to user
#[rule]
pub fn execute_donate_adds_tokens_no_shares(e: &Env, donor: &Address, pool: &Address, amount: i128) {
    cvlr_assume!(amount >= 0);
    let pb_before = storage::get_pool_balance(e, pool);
    let ub_before = storage::get_user_balance(e, pool, &donor);
    execute_donate(e, donor, pool, amount);
    let pb_after = storage::get_pool_balance(e, pool);
    let ub_after = storage::get_user_balance(e, pool, donor);
    cvlr_assert!(pb_after.tokens == pb_before.tokens + amount);
    cvlr_assert!(pb_after.shares == pb_before.shares);
    cvlr_assert!(pb_after.q4w == pb_before.q4w);
    cvlr_assert!(ub_after.shares == ub_before.shares);
}


// execute_donate should fail when pool has no shares and
// pool address wasn't deployed by the Pool Factory.
// using ghosts to avoid timeout
#[rule]
fn donate_not_from_factory_fails(e: &Env, user: &Address, pool: &Address, amount: i128) {
    unsafe {
        GHOST_IS_POOL.init(false);
    }

    let pb_before = storage::get_pool_balance(e, pool);

    cvlr_assume!(pb_before.shares == 0);
    execute_donate(e, user, pool, amount);
    unsafe {
        let is_pool = GHOST_IS_POOL.get();
        cvlr_assert!(is_pool);
    }
}

// 2. ASSERTIONS CHECKS

// execute_donate should fail if the amount < 0
#[rule]
fn donate_negative_amount_fails(e: &Env, from: &Address, pool: &Address, amount: i128) {
    cvlr_assume!(amount < 0);
    execute_donate(e, from, pool, amount);
    cvlr_assert!(false);
}

// execute_donate should fail when called from pool_address
#[rule]
fn execute_donate_from_pool_fails(e: &Env, pool: Address, amount: i128) {
    execute_donate(e, &pool, &pool, amount);
    cvlr_assert!(false); 
}

// execute_donate should fail when called from current_contract_address
#[rule]
fn execute_donate_from_contract_fails(e: &Env, pool: Address, amount: i128) {
    execute_donate(e, &e.current_contract_address(), &pool, amount);
    cvlr_assert!(false); 
}