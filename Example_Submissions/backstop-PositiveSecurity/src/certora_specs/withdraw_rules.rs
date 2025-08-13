use cvlr::{cvlr_assert, cvlr_assume};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, vec};

use crate::{backstop::{execute_queue_withdrawal, execute_withdraw, execute_dequeue_withdrawal}, storage};
use crate::constants::{MAX_Q4W_SIZE, Q4W_LOCK_TIME};

use crate::backstop::{GHOST_UB_BEFORE_W, GHOST_UB_AFTER_W, GHOST_UB_Q4W_BEFORE_W, GHOST_UB_Q4W_AFTER_W,
    GHOST_PB_TOKENS_BEFORE_W, GHOST_PB_SHARES_BEFORE_W, GHOST_PB_TOKENS_AFTER_W, GHOST_PB_SHARES_AFTER_W,
    GHOST_PB_Q4W_BEFORE_W, GHOST_PB_Q4W_AFTER_W};

// shares to withdraw must be nonnegative
#[rule]
pub fn amount_to_withdraw_nonnegative(e: &Env, from: Address, pool_address: Address, shares: i128) {
    cvlr_assume!(shares < 0);
    execute_withdraw(e, &from, &pool_address, shares);
    cvlr_assert!(false); // should pass when assumption is enabled, fail otherwise
}

// withdraw queue entries are all positive
// needs -smt_preciseBitwiseOps
#[rule]
pub fn withdraw_queue_only_positive(e: Env, from: Address, pool_address: Address, amount: i128) {
    cvlr_assume!(amount < 0);
    execute_queue_withdrawal(&e, &from, &pool_address, amount);
    cvlr_assert!(false); // should pass when assumption is enabled, fail otherwise
}

// RULES FOR PRIVATE MUTANTS
//
// execute_queue_withdrawal RULES
// 1. CORRECTNESS RULES

// execute_queue_withdrawal should decrease user's shares
// using ghosts to avoid timeout
#[rule]
pub fn execute_queue_withdrawal_decreases_ub(e: &Env, user: &Address, pool_address: &Address, amount: i128) {
    unsafe {
        GHOST_UB_BEFORE_W.init(0);
        GHOST_UB_AFTER_W.init(0);
    }

    cvlr_assume!(amount >= 0);
    
    execute_queue_withdrawal(e, user, pool_address, amount);
    
    unsafe {
        let ub_before = GHOST_UB_BEFORE_W.get();
        let ub_after = GHOST_UB_AFTER_W.get();
        cvlr_assert!(ub_after == ub_before - amount);
    }
}

// execute_queue_withdrawal should increase pool's q4w amount, shouldn't change shares and tokens amount
// using ghosts to avoid timeout
#[rule]
pub fn execute_queue_withdrawal_increases_pb_q4w_only(e: &Env, user: &Address, pool_address: &Address, amount: i128) {
    unsafe {
        GHOST_PB_TOKENS_BEFORE_W.init(0); 
        GHOST_PB_SHARES_BEFORE_W.init(0); 
        GHOST_PB_TOKENS_AFTER_W.init(0); 
        GHOST_PB_SHARES_AFTER_W.init(0);
        GHOST_PB_Q4W_BEFORE_W.init(0); 
        GHOST_PB_Q4W_AFTER_W.init(0);
    }

    cvlr_assume!(amount >= 0);

    execute_queue_withdrawal(e, user, pool_address, amount);

    unsafe {
        let pb_tokens_before = GHOST_PB_TOKENS_BEFORE_W.get(); 
        let pb_shares_before = GHOST_PB_SHARES_BEFORE_W.get();
        let pb_q4w_before = GHOST_PB_Q4W_BEFORE_W.get();  
        let pb_tokens_after = GHOST_PB_TOKENS_AFTER_W.get(); 
        let pb_shares_after = GHOST_PB_SHARES_AFTER_W.get();
        let pb_q4w_after = GHOST_PB_Q4W_AFTER_W.get();
        cvlr_assert!(pb_q4w_after == pb_q4w_before + amount);
        cvlr_assert!(pb_shares_after == pb_shares_before);
        cvlr_assert!(pb_tokens_after == pb_tokens_before);
    }
}

// execute_queue_withdrawal correctly adds a new q4w record
// 
#[rule]
pub fn execute_queue_withdrawal_updates_q4w_correct(e: &Env, user: &Address, pool_address: &Address, amount: i128) {
    cvlr_assume!(amount >= 0);
    let q = execute_queue_withdrawal(e, user, pool_address, amount);
    cvlr_assert!(q.amount == amount);
    cvlr_assert!(q.exp == e.ledger().timestamp() + Q4W_LOCK_TIME);
}

// 2. ASSERTIONS CHECKS

// execute_queue_withdrawal should fail if requested amount greater than users shares balance
#[rule]
pub fn execute_queue_balance_error_fails(e: &Env, user: &Address, pool_address: &Address, amount: i128) {
    unsafe {
        GHOST_UB_BEFORE_W.init(0);
    }
    execute_queue_withdrawal(e, user, pool_address, amount);
    unsafe {
        let ub_before = GHOST_UB_BEFORE_W.get();
        cvlr_assert!(ub_before >= amount);
    }
}

// execute_queue_withdrawal should fail if the maximum amount of active Q4W entries reached
#[rule]
pub fn execute_queue_too_many_q4w_fails(e: &Env, user: &Address, pool_address: &Address, amount: i128) {
    let ub_before = storage::get_user_balance(e, pool_address, user);
    cvlr_assume!(ub_before.q4w.len() >= MAX_Q4W_SIZE);
    execute_queue_withdrawal(e, user, pool_address, amount);
    cvlr_assert!(false);
}

// execute_dequeue_withdrawal RULES
// 1. CORRECTNESS RULES

// execute_dequeue_withdrawal moves requested amount of shares from user's q4w to his balance
#[rule]
pub fn execute_dequeue_withdrawal_ub_shares_to_q4w(e: &Env, user: &Address, pool: &Address, amount: i128) {
    unsafe {
        GHOST_UB_BEFORE_W.init(0);
        GHOST_UB_Q4W_BEFORE_W.init(vec![e]);
        GHOST_UB_AFTER_W.init(0);
        GHOST_UB_Q4W_AFTER_W.init(vec![e]);
    }

    cvlr_assume!(amount >= 0);
    
    execute_dequeue_withdrawal(e, user, pool, amount);
    
    unsafe {
        let ub_before = GHOST_UB_BEFORE_W.get();
        let ub_q4w_before = GHOST_UB_Q4W_BEFORE_W.get();
        let queued_before: i128 = ub_q4w_before.iter().map(|x| x.amount).sum();
        
        let ub_after = GHOST_UB_AFTER_W.get();
        let ub_q4w_after = GHOST_UB_Q4W_AFTER_W.get();
        let queued_after: i128 = ub_q4w_after.iter().map(|x| x.amount).sum();

        cvlr_assert!(ub_after == ub_before + amount);
        cvlr_assert!(queued_after == queued_before - amount);
    }
}

// execute_dequeue_withdrawal should decrease pool's q4w amount, shares amount remains same
#[rule]
pub fn execute_dequeue_withdrawal_decreases_pb_q4w_only(e: &Env, user: &Address, pool: &Address, amount: i128) {
    unsafe {
        GHOST_PB_TOKENS_BEFORE_W.init(0); 
        GHOST_PB_SHARES_BEFORE_W.init(0); 
        GHOST_PB_TOKENS_AFTER_W.init(0); 
        GHOST_PB_SHARES_AFTER_W.init(0);
        GHOST_PB_Q4W_BEFORE_W.init(0); 
        GHOST_PB_Q4W_AFTER_W.init(0);
    }

    cvlr_assume!(amount >= 0);

    execute_dequeue_withdrawal(e, user, pool, amount);

    unsafe {
        let pb_tokens_before = GHOST_PB_TOKENS_BEFORE_W.get(); 
        let pb_shares_before = GHOST_PB_SHARES_BEFORE_W.get();
        let pb_q4w_before = GHOST_PB_Q4W_BEFORE_W.get();  
        let pb_tokens_after = GHOST_PB_TOKENS_AFTER_W.get(); 
        let pb_shares_after = GHOST_PB_SHARES_AFTER_W.get();
        let pb_q4w_after = GHOST_PB_Q4W_AFTER_W.get();
        cvlr_assert!(pb_q4w_after == pb_q4w_before - amount);
        cvlr_assert!(pb_shares_after == pb_shares_before);
        cvlr_assert!(pb_tokens_after == pb_tokens_before);
    }
}

// 2. ASSERTIONS CHECKS

// execute_dequeue_withdrawal entries are all positive
#[rule]
pub fn withdraw_dequeue_only_positive(e: Env, from: Address, pool_address: Address, amount: i128) {
    cvlr_assume!(amount < 0);
    execute_dequeue_withdrawal(&e, &from, &pool_address, amount);
    cvlr_assert!(false); // should pass when assumption is enabled, fail otherwise
}

// execute_dequeue_withdrawal should fail if requested amount is greater than pools q4w
#[rule]
pub fn withdraw_dequeue_insufficient_balance_fails(e: &Env, user: &Address, pool_address: &Address, amount: i128) {
    unsafe {
        GHOST_PB_Q4W_BEFORE_W.init(0);
    }
    execute_dequeue_withdrawal(e, user, pool_address, amount);
    unsafe {
        let pb_before_q4w = GHOST_PB_Q4W_BEFORE_W.get();
        cvlr_assert!(pb_before_q4w >= amount); 
    }
}

// 2. ASSERTION CHECKS

// execute_withdraw should fail if pool has no tokens
#[rule]
pub fn withdraw_empty_pool_fails(e: &Env, from: Address, pool_address: Address, shares: i128) {
    unsafe {
        GHOST_PB_TOKENS_BEFORE_W.init(0); 
    }

    execute_withdraw(e, &from, &pool_address, shares);
    
    unsafe {
        let pb_tokens_before = GHOST_PB_TOKENS_BEFORE_W.get(); 
        if pb_tokens_before == 0 {
            cvlr_assert!(false);
        } 
    }
}

// execute_withdraw should fail if pool has no tokens
#[rule]
pub fn execute_withdraw_balance_error_fails(e: &Env, from: Address, pool_address: Address, shares: i128) {
    unsafe {
        GHOST_PB_SHARES_BEFORE_W.init(0);
        GHOST_PB_Q4W_BEFORE_W.init(0); 
    }

    execute_withdraw(e, &from, &pool_address, shares);
    
    unsafe {
        let pb_shares_before = GHOST_PB_SHARES_BEFORE_W.get();
        let pb_q4w_before = GHOST_PB_Q4W_BEFORE_W.get();
        cvlr_assert!(shares <= pb_shares_before && shares <= pb_q4w_before);
    }
}