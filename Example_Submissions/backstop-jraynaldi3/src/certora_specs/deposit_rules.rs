use cvlr::{cvlr_assert, cvlr_assume,clog};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, String};
use crate::backstop::{execute_deposit};
use crate::certora_specs::summaries::storage::{init_ghost, init_user_ghost};
use crate::certora_specs::{
    summaries::storage, 
    mocks::{
        pool_factory::PoolFactoryClient, 
        pool_factory::init_is_pool_ghost,
        token::TokenWrapper as TokenClient
    }
};

/// execute_deposit must be non negative amount
#[rule]
pub fn execute_deposit_non_negative(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);

    execute_deposit(e, from, pool_address, amount);
    cvlr_assert!(amount >=0);
}

/// execute_deposit must be non zero return
#[rule]
pub fn execute_deposit_non_zero_return(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);

    let to_mint = execute_deposit(e, from, pool_address, amount);
    cvlr_assert!(to_mint > 0);
}

/// execute_deposit from must be valid address cannot be pool_address
#[rule]
pub fn execute_deposit_from_not_pool(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);
    let is_pool = from == pool_address;

    execute_deposit(e, from, pool_address, amount);
    cvlr_assert!(!is_pool);
}

/// execute_deposit from must be valid address cannot be this contract
#[rule]
pub fn execute_deposit_from_not_this_contract(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);
    let is_this_contract = from == &e.current_contract_address();

    execute_deposit(e, from, pool_address, amount);
    cvlr_assert!(!is_this_contract);
}

/// execute_deposit must add pool tokens, shares and q4w
#[rule]
pub fn execute_deposit_add_pool_tokens_shares(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);
    
    let mut pool_balance_before = storage::get_pool_balance(e, pool_address).clone();
    
    cvlr_assume!( pool_balance_before.shares >= 0
        && pool_balance_before.q4w >= 0
        && pool_balance_before.tokens>= 0
    );

    let to_mint = execute_deposit(e, from, pool_address, amount);

    let mut pool_balance_after = storage::get_pool_balance(e, pool_address).clone();

    cvlr_assert!(
        pool_balance_after.shares == pool_balance_before.shares + to_mint
        && pool_balance_after.tokens == pool_balance_before.tokens + amount
    );
}

/// execute_deposit should add user shares
#[rule]
pub fn execute_deposit_add_user_shares(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);

    let user_balance_before = storage::get_user_balance(e, pool_address, from).clone();
    cvlr_assume!( 
        user_balance_before.shares >= 0
        && amount >= 0
    );
    let to_mint = execute_deposit(e, from, pool_address, amount);

    let user_balance_after = storage::get_user_balance(e, pool_address, from).clone();

    cvlr_assert!(
        user_balance_after.shares == user_balance_before.shares + to_mint
    );
}

/// execute_deposit should check is from pool factory
#[rule]
pub fn execute_deposit_pool_factory_check(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);
    init_is_pool_ghost(pool_address);
    let mut pool_balance = storage::get_pool_balance(e, pool_address).clone();
    cvlr_assume!(pool_balance.shares == 0);
    let pool_factory_client = PoolFactoryClient::new(e, &storage::get_pool_factory(e));
    let is_pool = pool_factory_client.is_pool(pool_address);

    execute_deposit(e, from, pool_address, amount);
    
    cvlr_assert!(
        is_pool
    );
}

/// execute_deposit transfer from user 
#[rule]
pub fn execute_deposit_transfer_backstop_from(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);
    let backstop_token_client = TokenClient::new(e, &storage::get_backstop_token(e));
    backstop_token_client.init_balance(from);
    cvlr_assume!(amount >= 0);
    cvlr_assume!(from != pool_address);
    cvlr_assume!(from != &e.current_contract_address());

    let balance_before = backstop_token_client.balance(from);
    cvlr_assume!(balance_before >=0);

    execute_deposit(e, from, pool_address, amount);

    let balance_after = backstop_token_client.balance(from);

    cvlr_assert!( balance_after == balance_before - amount); 
}

/// execute_deposit transfer to this contract
#[rule]
pub fn execute_deposit_transfer_backstop_contract(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    init_ghost(from, pool_address);
    let backstop_token_client = TokenClient::new(e, &storage::get_backstop_token(e));
    backstop_token_client.init_balance(&e.current_contract_address());
    cvlr_assume!(amount >= 0);
    cvlr_assume!(from != pool_address);
    cvlr_assume!(from != &e.current_contract_address());

    // let balance_before = backstop_token_client.balance(from);
    let contract_balance_before = backstop_token_client.balance(&e.current_contract_address());

    execute_deposit(e, from, pool_address, amount);

    let contract_balance_after = backstop_token_client.balance(&e.current_contract_address());

    cvlr_assert!( contract_balance_after == contract_balance_before + amount); 
}
