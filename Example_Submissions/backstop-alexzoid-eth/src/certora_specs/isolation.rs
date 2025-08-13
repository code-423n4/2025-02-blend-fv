// Remove `certora_storage_ghost` just feature for rules in this file

use soroban_sdk::{Env, Address};
use cvlr::{cvlr_assert, cvlr_assume};
use crate::backstop::{PoolBalance, UserBalance};
use cvlr_soroban::nondet_address;
// User real storage instead of storage_ghost
use crate::storage;

// Interaction with pool must not touch another pool 
pub fn isolation_pool(
    e: &Env, 
    pool: &Address, 
    user: &Address, 
    _amount: i128,
    call_fn: impl FnOnce()
) {
    let pool_other = nondet_address(); 
    cvlr_assume!(pool != &pool_other);

    let pb_other_before: PoolBalance = storage::get_pool_balance(&e, &pool_other);
    let ub_pool_other_before: UserBalance = storage::get_user_balance(&e, &pool_other, user);

    call_fn();
    
    let pb_other_after: PoolBalance = storage::get_pool_balance(&e, &pool_other);
    let ub_pool_other_after: UserBalance = storage::get_user_balance(&e, &pool_other, user);

    // Another pool balance not change
    cvlr_assert!(pb_other_before.shares == pb_other_after.shares);
    cvlr_assert!(pb_other_before.tokens == pb_other_after.tokens);
    cvlr_assert!(pb_other_before.q4w == pb_other_after.q4w);

    // User in another pool balance not change
    cvlr_assert!(ub_pool_other_before.shares == ub_pool_other_after.shares);
    cvlr_assert!(ub_pool_other_before.q4w.len() == ub_pool_other_after.q4w.len());
}

// Interaction with user must not touch another user 
pub fn isolation_user(
    e: &Env, 
    pool: &Address, 
    user: &Address, 
    _amount: i128,
    call_fn: impl FnOnce()
) {
    let user_other = nondet_address(); 
    cvlr_assume!(user != &user_other);

    let ub_other_before: UserBalance = storage::get_user_balance(&e, pool, &user_other);

    call_fn();
    
    let ub_other_after: UserBalance = storage::get_user_balance(&e, pool, &user_other);

    // Another user in this pool not change
    cvlr_assert!(ub_other_before.shares == ub_other_after.shares);
    cvlr_assert!(ub_other_before.q4w.len() == ub_other_after.q4w.len());
}