use cvlr::{clog, cvlr_assert, cvlr_assume};
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};
use sep_41_token::TokenClient;
use crate::contract;
use crate::storage;
use crate::dependencies::PoolFactoryClient;
use crate::certora_specs::GHOST_IS_POOL; //@audit added for ghost
use crate::certora_specs::GHOST_UPDATE_EMISSIONS_CALLED; //@audit added for ghost
use crate::certora_specs::{GHOST_FROM_BALANCE, GHOST_TO_BALANCE}; //@audit added for ghost


//functions to call
use crate::backstop::execute_deposit;

//reusable functions
//three addresses not the same
pub fn three_addresses_not_same(_e: &Env, a: &Address, b: &Address, c: &Address) {
    cvlr_assume!(a != b && a != c && b != c);
}

//three amounts less than 2^32
pub fn three_amounts_less_than_2_32(_e: &Env, a: i128, b: i128, c: i128) {
    cvlr_assume!(a < 2^32 && b < 2^32 && c < 2^32);
    cvlr_assume!(a > 0 && b > 0 && c > 0);
    cvlr_assume!(a + b + c < 2^32);
}


// execute_deposit(): reverts if amount to mint is 0
#[rule]
pub fn deposit_amount_to_mint_zero(e: &Env) {
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let amount: i128 = cvlr::nondet();

    let pool_balance = storage::get_pool_balance(e, &pool_address);

    //set up
    let to_mint = pool_balance.convert_to_shares(amount);
    cvlr_assume!(to_mint <= 0); // ensure the function reverts

    //function call
    execute_deposit(&e, &from, &pool_address, amount);

    //assert this is never reached
    cvlr_assert!(false);
}

//execute_deposit(): reverts if pool_address is not from poolFactory
#[rule]
pub fn deposit_pool_address_not_from_pool_factory(e: &Env) {
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let amount: i128 = cvlr::nondet();

    //setup
    let pool_balance = storage::get_pool_balance(e, &pool_address);
    cvlr_assume!(pool_balance.shares == 0); // needed to enter the if statement

    //function call
    execute_deposit(e, &from, &pool_address, amount);
    cvlr_assume!(unsafe{GHOST_IS_POOL == false});
    cvlr_assert!(false); //should never be reached
}

// execute_deposit(): increases total shares of pool
#[rule]
pub fn deposit_shares_increases_pool_shares(e: &Env) {
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let amount: i128 = cvlr::nondet();
    cvlr_assume!(amount < 100); // to prevent timeout

    //values before
    let pool_shares_before = storage::get_pool_balance(e, &pool_address).shares;
    
    //LOG
    clog!(amount as i64);
    clog!(pool_shares_before as i64);

    //function call
    let shares_to_mint = execute_deposit(&e, &from, &pool_address, amount);

    //values after
    let pool_shares_after = storage::get_pool_balance(e, &pool_address).shares;
    clog!(pool_shares_after as i64);

    //assert
    cvlr_assert!(pool_shares_after == pool_shares_before + shares_to_mint);
}

// execute_deposit(): increases shares of user
#[rule]
pub fn deposit_shares_increases_user_shares(e: &Env) {
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let amount: i128 = cvlr::nondet();
    cvlr_assume!(amount < 100); // to prevent timeout

    //values before
    let from_shares_before = storage::get_user_balance(e, &pool_address, &from).shares; 

    //function call
    let shares_to_mint = execute_deposit(&e, &from, &pool_address, amount);

    //values after
    let from_shares_after = storage::get_user_balance(e, &pool_address, &from).shares; //@audit only pool check
    clog!(from_shares_after as i64);

    //assert
    cvlr_assert!(from_shares_after == from_shares_before + shares_to_mint);
}

// execute_deposit(): increases tokens of pool
#[rule]
pub fn deposit_pool_tokens_change(e: &Env) {
    let from : Address = nondet_address();
    let pool_address : Address = nondet_address();
    let amount : i128 = cvlr::nondet();
    cvlr_assume!(amount > 0 && amount < 100); // to prevent timeout

    //values before
    let pool_tokens_before = storage::get_pool_balance(e, &pool_address).tokens;

    //function call
    execute_deposit(&e, &from, &pool_address, amount);

    //values after
    let pool_tokens_after = storage::get_pool_balance(e, &pool_address).tokens;

    //assert
    cvlr_assert!(pool_tokens_after == pool_tokens_before + amount);
}

// execute_deposit(): returns right amount of shares to mint
#[rule]
pub fn deposit_shares_to_mint(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    //setup
    let pool_balance = storage::get_pool_balance(e, pool_address);
    let target_shares_to_mint = pool_balance.convert_to_shares(amount);

    //function call
    let shares_to_mint = execute_deposit(&e, &from, &pool_address, amount);

    //assert
    cvlr_assert!(
        shares_to_mint == target_shares_to_mint
    );
}

//execute_deposit(): increaes the amount of tokens in the pool
#[rule]
pub fn deposit_balances_change(e: &Env) {
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let amount: i128 = cvlr::nondet(); 
    cvlr_assume!(amount == 1);
    
    //setup
    
    //set balances before
    let from_balance_before :i128 = cvlr::nondet();
    cvlr_assume!(from_balance_before < 100);
    unsafe {GHOST_FROM_BALANCE = from_balance_before};
    let from_balance_before_ghost = unsafe{GHOST_FROM_BALANCE};

    let contract_balance_before :i128 = cvlr::nondet();
    cvlr_assume!(contract_balance_before < 100);
    unsafe {GHOST_TO_BALANCE = contract_balance_before};
    let contract_balance_before_ghost = unsafe {GHOST_TO_BALANCE};

    //function call
    execute_deposit(&e, &from, &pool_address, amount);

    //values after
    let from_balance_after_ghost = unsafe {GHOST_FROM_BALANCE};
    let contract_balance_after_ghost = unsafe {GHOST_TO_BALANCE};

    //LOG:
    clog!(amount as i64);
    clog!(contract_balance_before_ghost as i64);
    clog!(contract_balance_after_ghost as i64);

    //assert
    cvlr_assert!(from_balance_after_ghost == from_balance_before_ghost - amount);
    cvlr_assert!(contract_balance_after_ghost == contract_balance_before_ghost + amount);
}

// execute_deposit(): updates emissions was called
#[rule]
pub fn deposit_update_emissions_called(e: &Env) {
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let amount: i128 = cvlr::nondet();
    // // ensure the call enters the if statement
    // let pool_balance = storage::get_pool_balance(e, &pool_address);
    // cvlr_assume!(pool_balance.shares == 0);
    
    //function call
    execute_deposit(&e, &from, &pool_address, amount);
    //assert that update_emissions was called

    cvlr_assert!(unsafe{GHOST_UPDATE_EMISSIONS_CALLED == true});


}


// execute_deposit(): reverts if amount is negativ
    #[rule]
pub fn deposit_amount_negative(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    //setup
    cvlr_assume!(amount < 0);

    //function call
    execute_deposit(&e, &from, &pool_address, amount);

    //assert this is never reached
    cvlr_assert!(false);
}

// execute_deposit(): reverts if from address is pool_address or e.current_contract_address()
#[rule]
pub fn deposit_from_pool_address(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    //setup
    cvlr_assume!(from == pool_address || from == &e.current_contract_address());

    //function call
    execute_deposit(&e, &from, &pool_address, amount);

    //assert this is never reached
    cvlr_assert!(false);
}


