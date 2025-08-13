use cvlr::{cvlr_assert, cvlr_assume,clog};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};
use crate::backstop::{execute_draw, execute_donate};
use crate::certora_specs::mocks::pool_factory::is_pool_ghost;
use crate::certora_specs::summaries::storage::{init_ghost, init_user_ghost};
use crate::certora_specs::{
    summaries::storage, 
    mocks::{
        pool_factory::PoolFactoryClient, 
        pool_factory::init_is_pool_ghost,
        token::TokenWrapper as TokenClient
    }
};

/* -------------------------------------------------------------------------- */
/*                                execute_draw                                */
/* -------------------------------------------------------------------------- */

/// execute_draw require non negative amount 
#[rule]
pub fn execute_draw_non_negative(e: &Env, pool_address: &Address, amount: i128, to: &Address) {
    execute_draw(e, pool_address, amount, to);
    cvlr_assert!(amount >= 0);
}

/// execute_draw withdraw pool_balance tokens
#[rule]
pub fn execute_draw_withdraw_tokens(e: &Env, pool_address: &Address, amount: i128, to: &Address) {
    init_ghost(to, pool_address);
    let pool_balance_before = storage::get_pool_balance(e, pool_address);
    execute_draw(e, pool_address, amount, to);
    let pool_balance_after = storage::get_pool_balance(e, pool_address);
    cvlr_assert!(pool_balance_after.tokens == pool_balance_before.tokens - amount);
}

/// execute_draw transfer from current contract 
#[rule]
pub fn execute_draw_transfer_from_contract(e: &Env, pool_address: &Address, amount: i128, to: &Address) {
    let backstop_token_client = TokenClient::new(e, &storage::get_backstop_token(e));
    let contract_addr = &e.current_contract_address();
    backstop_token_client.init_balance(contract_addr);    
    let is_this_contract = to == &e.current_contract_address();
    cvlr_assume!(!is_this_contract);
    
    let contract_balance_before = backstop_token_client.balance(contract_addr);
    cvlr_assume!(contract_balance_before >= 0);
    clog!(contract_balance_before.clamp(i64::MIN as i128, i64::MAX as i128) as i64);

    execute_draw(e, pool_address, amount, to);

    let contract_balance_after = backstop_token_client.balance(contract_addr);
    clog!(contract_balance_after.clamp(i64::MIN as i128, i64::MAX as i128) as i64);

    cvlr_assert!(contract_balance_after == contract_balance_before - amount);
}

/// execute_draw transfer to "to"
#[rule]
pub fn execute_draw_transfer_to(e: &Env, pool_address: &Address, amount: i128, to: &Address) {
    init_ghost(to, pool_address);
    let backstop_token_client = TokenClient::new(e, &storage::get_backstop_token(e));
    backstop_token_client.init_balance(to);
    let is_this_contract = to == &e.current_contract_address();
    cvlr_assume!(!is_this_contract);

    let balance_before = backstop_token_client.balance(to);
    clog!(balance_before.clamp(i64::MIN as i128, i64::MAX as i128) as i64);

    execute_draw(e, pool_address, amount, to);

    let balance_after = backstop_token_client.balance(to);
    clog!(balance_after.clamp(i64::MIN as i128, i64::MAX as i128) as i64);

    cvlr_assert!(balance_after == balance_before + amount);
}

/* -------------------------------------------------------------------------- */
/*                               execute_donate                               */
/* -------------------------------------------------------------------------- */
/// execute_donate require non negative amount 
#[rule]
pub fn execute_donate_non_negative(e: &Env, from: &Address, pool_address: &Address, amount: i128){
    execute_donate(e, from, pool_address, amount);
    cvlr_assert!(amount >= 0);
}

/// execute_donate not from pool address
#[rule]
pub fn execute_donate_from_not_pool(e: &Env, from: &Address, pool_address: &Address, amount: i128){
    let is_pool_address = from == pool_address;
    execute_donate(e, from, pool_address, amount);
    cvlr_assert!(!is_pool_address);
}

/// execute_donate not from this contract
#[rule]
pub fn execute_donate_from_not_this_contract(e: &Env, from: &Address, pool_address: &Address, amount: i128){
    let is_this_contract = from == &e.current_contract_address();
    execute_donate(e, from, pool_address, amount);
    cvlr_assert!(!is_this_contract);
}

/// execute_donate should check is from pool factory
#[rule]
pub fn execute_donate_is_from_pool_factory_check(e: &Env, from: &Address, pool_address: &Address, amount: i128){
    init_is_pool_ghost(pool_address);
    init_ghost(from, pool_address);
    let pool_balance = storage::get_pool_balance(e, pool_address);
    let pool_factory_client = PoolFactoryClient::new(e, &storage::get_pool_factory(e));
    let is_pool = pool_factory_client.is_pool(pool_address);
    cvlr_assume!(pool_balance.shares == 0);

    execute_donate(e, from, pool_address, amount);

    cvlr_assert!(is_pool);
}

/// execute_donate should reduce allowance of "from" to contract
#[rule]
pub fn execute_donate_reduce_allowance(e: &Env, from: &Address, pool_address: &Address, amount: i128){
    let backstop_token = TokenClient::new(e, &storage::get_backstop_token(e));
    backstop_token.init_allowance(from, &e.current_contract_address());

    let allowance_before = backstop_token.allowance(from, &e.current_contract_address());
    execute_donate(e, from, pool_address, amount);

    let allowance_after = backstop_token.allowance(from, &e.current_contract_address());
    cvlr_assert!(allowance_after == allowance_before - amount);
}

/// execute_donate should transfer token from "from"
#[rule]
pub fn execute_donate_transfer_from(e: &Env, from: &Address, pool_address: &Address, amount: i128){
    let backstop_token = TokenClient::new(e, &storage::get_backstop_token(e));
    backstop_token.init_balance(from);

    let balance_before = backstop_token.balance(from);

    execute_donate(e, from, pool_address, amount);

    let balance_after = backstop_token.balance(from);

    cvlr_assert!(balance_after == balance_before - amount);
}

/// execute_donate should transfer token to this contract
#[rule]
pub fn execute_donate_transfer_to_this_contract(e: &Env, from: &Address, pool_address: &Address, amount: i128){
    let backstop_token = TokenClient::new(e, &storage::get_backstop_token(e));
    backstop_token.init_balance(&e.current_contract_address());

    let balance_before = backstop_token.balance(&e.current_contract_address());
    
    execute_donate(e, from, pool_address, amount);

    let balance_after = backstop_token.balance(&e.current_contract_address());

    cvlr_assert!(balance_after == balance_before + amount);
}

/// execute_donate should add pool tokens
#[rule]
pub fn execute_donate_add_pool_token(e: &Env, from: &Address, pool_address: &Address, amount: i128){
    init_ghost(from, pool_address);
    let pool_balance_before = storage::get_pool_balance(e, pool_address);
    
    execute_donate(e, from, pool_address, amount);

    let pool_balance_after = storage::get_pool_balance(e, pool_address);

    cvlr_assert!(pool_balance_after.tokens == pool_balance_before.tokens + amount);
}

/// contract should not changes pool q4w and shares
#[rule]
pub fn q4w_shares_not_changes(e: &Env, from: &Address, pool_address: &Address, amount: i128, to: &Address){
    init_ghost(from, pool_address);
    let pool_balance_before = storage::get_pool_balance(e, pool_address);
    let num:i8 = cvlr::nondet();

    // randomizer
    if num <= 10 {
        execute_donate(e, from, pool_address, amount);
    } else {
        execute_draw(e, pool_address, amount, to)
    }

    let pool_balance_after = storage::get_pool_balance(e, pool_address);

    cvlr_assert!(pool_balance_after.q4w == pool_balance_before.q4w);
    cvlr_assert!(pool_balance_after.shares == pool_balance_before.shares);
}
