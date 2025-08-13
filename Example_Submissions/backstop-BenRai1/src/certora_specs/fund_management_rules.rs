// use cvlr::clog;
use cvlr::cvlr_assert;
use cvlr::cvlr_assume;
use cvlr::nondet;
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};
use sep_41_token::TokenClient;

use crate::certora_specs::{GHOST_FROM_BALANCE, GHOST_TO_BALANCE, GHOST_ALLOWANCE}; 
use crate::storage;
use crate::{backstop::execute_draw, backstop::execute_donate};
use crate::certora_specs::GHOST_IS_POOL; //@audit added for ghost

//reusable functions
//three addresses not the same
pub fn three_addresses_not_same(_e: &Env, a: &Address, b: &Address, c: &Address) {
    cvlr_assume!(a != b && a != c && b != c);
}

//balances less than totla supply
pub fn balances_less_than_total_supply( a: i128, b: i128, c: i128) {
    let int_128_max = 2^64;
    cvlr_assume!(a < int_128_max && b < int_128_max && c < int_128_max);
    cvlr_assume!((a + b + c) == int_128_max);
}

// execute_draw() pool.shares and pool.q4w do not change
#[rule]
pub fn donate_pool_shares_q4w_not_change(e: &Env) {
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let amount: i128 = nondet();
    //setup
    let pool_balance = storage::get_pool_balance(e, &pool_address);
    let pool_shares_before = pool_balance.shares;
    let pool_q4w_before = pool_balance.q4w;

    //function call
    execute_donate(&e, &from, &pool_address, amount);

    //values after
    let pool_shares_after = storage::get_pool_balance(e, &pool_address).shares;
    let pool_q4w_after = storage::get_pool_balance(e, &pool_address).q4w;

    //assert
    cvlr_assert!(pool_shares_after == pool_shares_before);
    cvlr_assert!(pool_q4w_after == pool_q4w_before);
}

// execute_draw() pool.shares and pool.q4w do not change
#[rule]
pub fn draw_pool_shares_q4w_not_change(e: &Env) {
    let pool_address: Address = nondet_address();
    let amount: i128 = nondet();
    let to: Address = nondet_address();
    //setup
    let pool_balance = storage::get_pool_balance(e, &pool_address);
    let pool_shares_before = pool_balance.shares;
    let pool_q4w_before = pool_balance.q4w;

    //function call
    execute_draw(&e, &pool_address, amount, &to);

    //values after
    let pool_shares_after = storage::get_pool_balance(e, &pool_address).shares;
    let pool_q4w_after = storage::get_pool_balance(e, &pool_address).q4w;

    //assert
    cvlr_assert!(pool_shares_after == pool_shares_before);
    cvlr_assert!(pool_q4w_after == pool_q4w_before);
}

// execute_donate() revert if pool_address is not from the factory
#[rule]
pub fn donate_pool_address_not_from_pool_factory(e: &Env) { 
    let pool_address: Address = nondet_address();
    let from: Address = nondet_address();
    let amount: i128 = nondet();
    //setup
    let pool_balance = storage::get_pool_balance(e, &pool_address);
    cvlr_assume!(pool_balance.shares == 0); // needed to enter the if statement
    
    //function call
    execute_donate(&e, &from, &pool_address, amount);
    cvlr_assume!(unsafe{GHOST_IS_POOL == false});

    //assert this is never reached
    cvlr_assert!(false);
}
// execute_donate() reverts if amount is negative
#[rule]
pub fn donate_amount_negative(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    //setup
    cvlr_assume!(amount < 0);

    //function call
    execute_donate(&e, &from, &pool_address, amount);

    //assert this is never reached
    cvlr_assert!(false);
}

// execute_donate() reverts if from ==  pool_address or if from == e.current_contract_address
#[rule]
pub fn donate_from_pool_address(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    //setup
    cvlr_assume!(from == pool_address || from == &e.current_contract_address());

    //function call
    execute_donate(&e, &from, &pool_address, amount);

    //assert this is never reached
    cvlr_assert!(false);
}

// execute_donate() reduces the balance of from by amount, increases balance of current_contract by amount, does not change balance of other
#[rule]
pub fn donate_balance_change(e: &Env) {
    //setup
    let from: Address = nondet_address();
    let amount: i128 = nondet();
    let pool_address: Address = nondet_address();
    cvlr_assume!(from != e.current_contract_address());

    //setup
    let from_balance_before :i128 = cvlr::nondet();
    unsafe {GHOST_FROM_BALANCE = from_balance_before};
    let from_balance_before_ghost = unsafe {GHOST_FROM_BALANCE};

    let contract_balance_before :i128 = cvlr::nondet();
    unsafe {GHOST_TO_BALANCE = contract_balance_before};
    let contract_balance_before_ghost = unsafe {GHOST_TO_BALANCE};

    //function call
    execute_donate(&e, &from, &pool_address, amount);

    //values after
    let from_balance_after_ghost = unsafe {GHOST_FROM_BALANCE};
    let contract_balance_after_ghost = unsafe {GHOST_TO_BALANCE};

    //assert
    cvlr_assert!(from_balance_after_ghost == from_balance_before_ghost - amount);
    cvlr_assert!(contract_balance_after_ghost == contract_balance_before_ghost + amount);
}

// execute_donate() increases pool.tokens by amount
#[rule]
pub fn donate_pool_tokens_change(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    //values before
    let pool_tokens_before = storage::get_pool_balance(e, pool_address).tokens;

    //function call
    execute_donate(&e, &from, &pool_address, amount);

    //values after
    let pool_tokens_after = storage::get_pool_balance(e, pool_address).tokens;

    //assert
    cvlr_assert!(pool_tokens_after == pool_tokens_before + amount);
}

// execute_donate() reduced allowance of current_contract for from by amount if not infinit
#[rule]
pub fn donate_allowance_change(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    //values before
    let backstop_token = TokenClient::new(e, &storage::get_backstop_token(e));
    let allowance_before = backstop_token.allowance(&from, &e.current_contract_address());

    //function call
    execute_donate(&e, &from, &pool_address, amount);

    //values after
    let allowance_after = backstop_token.allowance(&from, &e.current_contract_address());

    //assert
    cvlr_assert!(allowance_after == allowance_before - amount); //@audit if not inifint einarbeiten
}


// execute_draw() reverts if amount is negativ
#[rule]
pub fn draw_amount_negative(e: &Env, pool_address: &Address, amount: i128, to: &Address) {
    //setup
    cvlr_assume!(amount < 0);

    //function call
    execute_draw(&e, &pool_address, amount, &to);

    //assert this is never reached
    cvlr_assert!(false);
}

// execute_draw() reverts if pool.tokens < amount
#[rule]
pub fn draw_pool_tokens_less_than_amount(e: &Env, pool_address: &Address, amount: i128, to: &Address) {
    //setup
    let pool_balance = storage::get_pool_balance(e, pool_address);
    cvlr_assume!(pool_balance.tokens < amount);

    //function call
    execute_draw(&e, &pool_address, amount, &to);

    //assert this is never reached
    cvlr_assert!(false);
}


// execute_draw() reduces balance of token (curreentConttact) by amount, increases balance of token (to) by amount and does not touch other
#[rule]
pub fn draw_balance_changes(e: &Env) {
    let pool_address: Address = nondet_address();
    let amount: i128 = nondet();
    let to: Address = nondet_address();
    cvlr_assume!(to != e.current_contract_address());


    //values before
    let contract_balance_before :i128 = cvlr::nondet();
    cvlr_assume!(contract_balance_before < 100);
    unsafe {GHOST_FROM_BALANCE = contract_balance_before};
    let contract_balance_before_ghost = unsafe {GHOST_FROM_BALANCE};

    let to_balance_before :i128 = cvlr::nondet();
    cvlr_assume!(to_balance_before < 100);
    unsafe {GHOST_TO_BALANCE = to_balance_before};
    let to_balance_before_ghost = unsafe {GHOST_TO_BALANCE};

    //function call
    execute_draw(&e, &pool_address, amount, &to);

    //values after
    let contract_balance_after_ghost = unsafe {GHOST_FROM_BALANCE};
    let to_balance_after_ghost = unsafe {GHOST_TO_BALANCE};

    //assert
    cvlr_assert!(contract_balance_after_ghost == contract_balance_before_ghost - amount);
    cvlr_assert!(to_balance_after_ghost == to_balance_before_ghost + amount);
}

// execute_draw() reduces pool.tokens for pool_address by amount, pool.amount for other pool is not changed
#[rule]
pub fn draw_pool_tokens_change(e: &Env) { 
    let pool_address: Address = nondet_address();
    let amount: i128 = nondet();
    let to: Address = nondet_address();
    // let other_pool_address: Address = nondet_address();
    //setup
    // cvlr_assume!(pool_address != other_pool_address);

    //values before
    let pool_tokens_before = storage::get_pool_balance(e, &pool_address).tokens;
    // let other_pool_tokens_before = storage::get_pool_balance(e, &other_pool_address).tokens;

    //function call
    execute_draw(&e, &pool_address, amount, &to);

    //values after
    let pool_tokens_after = storage::get_pool_balance(e, &pool_address).tokens;
    // let other_pool_tokens_after = storage::get_pool_balance(e, &other_pool_address).tokens;

    //assert
    cvlr_assert!(pool_tokens_after == pool_tokens_before - amount);
    // cvlr_assert!(other_pool_tokens_after == other_pool_tokens_before);
}

