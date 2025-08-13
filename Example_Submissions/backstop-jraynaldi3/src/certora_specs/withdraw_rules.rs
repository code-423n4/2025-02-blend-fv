use cvlr::{cvlr_assert, cvlr_assume,clog};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

// use crate::backstop::{execute_queue_withdrawal, execute_withdraw, UserBalance};
use crate::{backstop::{execute_queue_withdrawal, execute_withdraw, UserBalance, execute_dequeue_withdrawal}, constants::Q4W_LOCK_TIME};
use crate::certora_specs::{
    summaries::storage::{init_ghost}, 
    summaries::storage,
    mocks::token::TokenWrapper as TokenClient
};

/* -------------------------------------------------------------------------- */
/*                              execute_withdraw                              */
/* -------------------------------------------------------------------------- */
// shares to withdraw must be nonnegative
#[rule]
pub fn ew_amount_to_withdraw_nonnegative(e: &Env, from: &Address, pool_address: &Address, shares: i128) {
    init_ghost(from, pool_address);
    
    let user_balance_before = storage::get_user_balance(e, pool_address, from).clone();
    setup(user_balance_before.clone());
    
    execute_withdraw(e, from, pool_address, shares);
    cvlr_assert!(shares >= 0);
}

/// execute_withdraw pop first user q4w
#[rule]
pub fn ew_pop_q4w(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);
    
    let user_balance_before = storage::get_user_balance(e, pool_address, from).clone();
    cvlr_assume!(amount >= 0);
    cvlr_assume!(amount < i8::MAX as i128);
    setup(user_balance_before.clone());

    let first_q4w_amount_before = user_balance_before.q4w.first_unchecked().amount;
    cvlr_assume!(first_q4w_amount_before < amount);
    cvlr_assume!(first_q4w_amount_before >= 0);

    execute_withdraw(e, from, pool_address, amount);

    let user_balance_after = storage::get_user_balance(e, pool_address, from).clone();

    cvlr_assert!( user_balance_after.q4w.len() < 2); 
}

/// execute_withdraw transfer to from
#[rule]
pub fn ew_transfer_backstop_from(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    let backstop_token_client = TokenClient::new(e, &storage::get_backstop_token(e));
    backstop_token_client.init_balance(from);

    let is_this_contract = from == &e.current_contract_address();
    cvlr_assume!(!is_this_contract);
    cvlr_assume!(amount >= 0);

    let balance_before = backstop_token_client.balance(from);
    cvlr_assume!(balance_before >= 0);

    let return_value = execute_withdraw(e, from, pool_address, amount);

    let balance_after = backstop_token_client.balance(from);

    cvlr_assert!( balance_after == balance_before + return_value); 
}

/// execute_withdraw transfer from this contract
#[rule]
pub fn ew_transfer_backstop_contract(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    let backstop_token_client = TokenClient::new(e, &storage::get_backstop_token(e));
    backstop_token_client.init_balance(&e.current_contract_address());

    let is_this_contract = from == &e.current_contract_address();
    cvlr_assume!(!is_this_contract);
    cvlr_assume!(amount >= 0);

    let contract_balance_before = backstop_token_client.balance(&e.current_contract_address());
    cvlr_assume!(contract_balance_before >= amount);

    let return_value = execute_withdraw(e, from, pool_address, amount);

    let contract_balance_after = backstop_token_client.balance(&e.current_contract_address());

    cvlr_assert!( contract_balance_after == contract_balance_before - return_value); 
}

/// execute_withdraw revert if return == 0
#[rule]
pub fn ew_revert_zero_return(e: &Env, from: &Address, pool_address: &Address, amount: i128) {

    let return_value = execute_withdraw(e, from, pool_address, amount);
    cvlr_assume!(return_value == 0);
    cvlr_assert!(false); 
}

/* -------------------------------------------------------------------------- */
/*                          execute_queue_withdrawal                          */
/* -------------------------------------------------------------------------- */
// execute_queue_withdrawal entries are all positive
// needs -smt_preciseBitwiseOps
#[rule]
pub fn eqw_only_positive(e: Env, from: Address, pool_address: Address, amount: i128) {
    execute_queue_withdrawal(&e, &from, &pool_address, amount);
    cvlr_assert!(amount >= 0); 
}

/// execute_queue_withdrawal should reduce balance 
#[rule]
pub fn eqw_reduce_balance(e :&Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);
    
    let user_balance = storage::get_user_balance(e, pool_address, from).clone();
    cvlr_assume!(user_balance.shares >= 0 && amount >= 0);

    // cvlr_assume!(amount > user_balance.shares);
    execute_queue_withdrawal(e, from, pool_address, amount);

    let user_balance_after = storage::get_user_balance(e, pool_address, from).clone();

    cvlr_assert!(user_balance_after.shares == user_balance.shares - amount); 
}

/// execute_queue_withdrawal should return latest q4w that pushed
#[rule]
pub fn eqw_return_value(e :&Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);
    
    let user_balance = storage::get_user_balance(e, pool_address, from).clone();
    cvlr_assume!(user_balance.shares >= 0 && amount >= 0);

    // cvlr_assume!(amount > user_balance.shares);
    let last_q4w = execute_queue_withdrawal(e, from, pool_address, amount);


    cvlr_assert!(last_q4w.amount == amount && last_q4w.exp ==  e.ledger().timestamp() + Q4W_LOCK_TIME); 
}

/// execute_queue_withdrawal should return latest q4w that pushed
#[rule]
pub fn eqw_add_pool_queue(e :&Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);
    
    let user_balance = storage::get_user_balance(e, pool_address, from).clone();
    let pool_balance = storage::get_pool_balance(e, pool_address).clone();
    cvlr_assume!(user_balance.shares >= 0 && amount >= 0);

    // cvlr_assume!(amount > user_balance.shares);
    let last_q4w = execute_queue_withdrawal(e, from, pool_address, amount);


    let pool_balance_after = storage::get_pool_balance(e, pool_address).clone();


    cvlr_assert!(pool_balance_after.q4w == pool_balance.q4w + amount); 
}

/* -------------------------------------------------------------------------- */
/*                         execute_dequeue_withdrawal                         */
/* -------------------------------------------------------------------------- */

pub fn setup(user_balance:UserBalance){
    cvlr_assume!(user_balance.shares >= 0);
    cvlr_assume!(user_balance.q4w.len() == 2);
    cvlr_assume!(user_balance.q4w.first_unchecked().amount >= 0);
    cvlr_assume!(user_balance.q4w.last_unchecked().amount >= 0);
}

// execute_dequeue_withdrawal entries are all positive
// needs -smt_preciseBitwiseOps
#[rule]
pub fn edw_only_positive(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);
    let user_balance_before = storage::get_user_balance(e, pool_address, from).clone();
    setup(user_balance_before.clone());

    execute_dequeue_withdrawal(e, from, pool_address, amount);
    cvlr_assert!(amount >= 0); 
}

/// execute_dequeue_withdrawal should add user shares 
#[rule]
pub fn edw_add_user_shares(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);
    
    let user_balance_before = storage::get_user_balance(e, pool_address, from).clone();
    cvlr_assume!(amount >= 0);
    setup(user_balance_before.clone());

    // cvlr_assume!(amount > user_balance.shares);
    execute_dequeue_withdrawal(e, from, pool_address, amount);

    let user_balance_after = storage::get_user_balance(e, pool_address, from).clone();

    cvlr_assert!( user_balance_after.shares == user_balance_before.shares + amount); 
}

/// execute_dequeue_withdrawal pop last user q4w
#[rule]
pub fn edw_pop_q4w(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);
    
    let user_balance_before = storage::get_user_balance(e, pool_address, from).clone();
    cvlr_assume!(amount >= 0);
    setup(user_balance_before.clone());

    let last_q4w_amount_before = user_balance_before.q4w.last_unchecked().amount;
    cvlr_assume!(last_q4w_amount_before < amount);

    execute_dequeue_withdrawal(e, from, pool_address, amount);

    let user_balance_after = storage::get_user_balance(e, pool_address, from).clone();

    cvlr_assert!( user_balance_after.q4w.len() < 2); 
}

/// execute_dequeue_withdrawal cannot add shares more than q4w amount
#[rule]
pub fn edw_added_shares_lte_q4w_amount(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);
    
    let user_balance_before = storage::get_user_balance(e, pool_address, from).clone();
    cvlr_assume!(amount >= 0);
    setup(user_balance_before.clone());

    let total_q4w_before = user_balance_before.q4w.first_unchecked().amount + user_balance_before.q4w.last_unchecked().amount;

    // cvlr_assume!(amount > user_balance.shares);
    execute_dequeue_withdrawal(e, from, pool_address, amount);

    let user_balance_after = storage::get_user_balance(e, pool_address, from).clone();

    cvlr_assert!( user_balance_after.shares - user_balance_before.shares <= total_q4w_before); 
}

/// execute_dequeue_withdrawal reduce pool q4w
#[rule]
pub fn edw_reduce_pool_q4w(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);
    
    let user_balance_before = storage::get_user_balance(e, pool_address, from).clone();
    cvlr_assume!(amount >= 0);
    setup(user_balance_before.clone());

    let pool_balance_before = storage::get_pool_balance(e, pool_address).clone();

    // cvlr_assume!(amount > user_balance.shares);
    execute_dequeue_withdrawal(e, from, pool_address, amount);

    let pool_balance_after = storage::get_pool_balance(e, pool_address).clone();

    cvlr_assert!( pool_balance_after.q4w == pool_balance_before.q4w - amount); 
}

/// execute_dequeue_withdrawal reduce pool q4w
#[rule]
pub fn edw_pool_q4w_non_negative(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);
    
    let user_balance_before = storage::get_user_balance(e, pool_address, from).clone();
    cvlr_assume!(amount >= 0);
    setup(user_balance_before.clone());

    // cvlr_assume!(amount > user_balance.shares);
    execute_dequeue_withdrawal(e, from, pool_address, amount);

    let pool_balance_after = storage::get_pool_balance(e, pool_address).clone();

    cvlr_assert!( pool_balance_after.q4w >= 0); 
}