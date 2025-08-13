use cvlr::cvlr_assert;
use cvlr::cvlr_assume;
use cvlr_soroban_derive::rule;
use soroban_fixed_point_math::FixedPoint;
use soroban_sdk::{Address, Env, unwrap::UnwrapOptimized};

use crate::{backstop::execute_deposit, storage};

// ghosts for verification purposes
#[cfg(feature = "certora")]
use crate::{certora_specs::mocks::pool_factory::GHOST_IS_POOL,
    backstop::{GHOST_UB_BEFORE, GHOST_UB_AFTER, 
    GHOST_PB_SHARES_BEFORE, GHOST_PB_SHARES_AFTER, GHOST_PB_TOKENS_BEFORE, GHOST_PB_TOKENS_AFTER}};


// RULES FOR PRIVATE MUTANTS
//
// execute_deposit RULES
// 1. CORRECTNESS RULES

// execute_deposit returns positive amount of minted shares
#[rule]
pub fn execute_deposit_mints_positive_shares(e: &Env, user: &Address, pool_address: &Address, amount: i128) {
    cvlr_assume!(amount > 0);
    let minted_shares = execute_deposit(e, user, pool_address, amount);
    cvlr_assert!(minted_shares > 0);
}

// execute_deposit correctly increases user balance
// using ghosts to avoid timeout
#[rule]
pub fn execute_deposit_ub_correct(e: &Env, user: &Address, pool_address: &Address, amount: i128) {
    unsafe {
        GHOST_UB_BEFORE.init(0);
        GHOST_UB_AFTER.init(0);
    }
    cvlr_assume!(amount > 0);
    let minted_shares = execute_deposit(e, user, pool_address, amount);
    
    unsafe {
        let ub_before = GHOST_UB_BEFORE.get();
        let ub_after = GHOST_UB_AFTER.get();
        cvlr_assert!(ub_after == ub_before + minted_shares);
    }
}

// execute_deposit correctly increases pool balance
// using ghosts to avoid timeout
#[rule]
pub fn execute_deposit_pb_correct(e: &Env, user: &Address, pool_address: &Address, amount: i128) {
    unsafe {
        GHOST_PB_SHARES_BEFORE.init(0);
        GHOST_PB_SHARES_AFTER.init(0);
        GHOST_PB_TOKENS_BEFORE.init(0);
        GHOST_PB_TOKENS_AFTER.init(0);
    }

    cvlr_assume!(amount > 0);
    let minted_shares = execute_deposit(e, user, pool_address, amount);

    unsafe {
        let shares_before = GHOST_PB_SHARES_BEFORE.get();
        let shares_after = GHOST_PB_SHARES_AFTER.get();
        let tokens_before = GHOST_PB_TOKENS_BEFORE.get();
        let tokens_after = GHOST_PB_TOKENS_AFTER.get();
        cvlr_assert!(shares_after == shares_before + minted_shares);
        cvlr_assert!(tokens_after == tokens_before + amount);
    }
}

// execute_deposit correctly calculates shares if pool has shares
// using ghosts to avoid timeout
#[rule]
fn deposit_calculates_correct_shares(e: &Env, user: &Address, pool: &Address, amount: i128) {
    unsafe {
        GHOST_PB_SHARES_BEFORE.init(0);
        GHOST_PB_TOKENS_BEFORE.init(0);
    }

    cvlr_assume!(amount > 0);

    let to_mint = execute_deposit(e, user, pool, amount);
    
    unsafe {
        let shares_before = GHOST_PB_SHARES_BEFORE.get();
        let tokens_before = GHOST_PB_TOKENS_BEFORE.get();
        let expected = amount.fixed_mul_floor(shares_before, tokens_before).unwrap_optimized();
        if shares_before > 0 {
            cvlr_assert!(to_mint == expected);
        }
    }
}

// execute_deposit mints `amount` shares if pool has no shares;
// also should fail when pool address wasn't deployed by the Pool Factory.
// using ghosts to avoid timeout
#[rule]
fn deposit_calculates_correct_shares_0(e: &Env, user: &Address, pool: &Address, amount: i128) {
    unsafe {
        GHOST_PB_SHARES_BEFORE.init(0);
        GHOST_IS_POOL.init(false);
    }

    cvlr_assume!(amount > 0);
    let to_mint = execute_deposit(e, user, pool, amount);
    unsafe {
        let shares_before = GHOST_PB_SHARES_BEFORE.get();
        let is_pool = GHOST_IS_POOL.get();
        if shares_before == 0 {
            cvlr_assert!(to_mint == amount);
            cvlr_assert!(is_pool);
        }
    }
}

// 2. ASSERTION CHECKS

// execute_deposit should fail if amount <= 0
#[rule]
pub fn execute_deposit_negative_amount_fails(e: &Env, from: Address, pool_address: Address, amount: i128) {
    cvlr_assume!(amount <= 0);
    execute_deposit(&e, &from, &pool_address, amount);
    cvlr_assert!(false);
}

// execute_deposit should fail when called from pool_address
#[rule]
fn execute_deposit_from_pool_fails(e: &Env, pool: Address, amount: i128) {
    execute_deposit(e, &pool, &pool, amount);
    cvlr_assert!(false); 
}

// execute_deposit should fail when called from current_contract_address
#[rule]
fn execute_deposit_from_contract_fails(e: &Env, pool: Address, amount: i128) {
    execute_deposit(e, &e.current_contract_address(), &pool, amount);
    cvlr_assert!(false); 
}
