
use cvlr::cvlr_assert;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::{backstop::execute_deposit, storage};

// outer deposit function increases pool balance
#[rule]
pub fn outer_deposit(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    let pool_balance_before = storage::get_pool_balance(e, pool_address);
    let pool_tokens_before = pool_balance_before.tokens;
    let _to_mint = execute_deposit(&e, &from, &pool_address, amount);
    let pool_balance_after = storage::get_pool_balance(e, pool_address);
    let pool_tokens_after = pool_balance_after.tokens;
    cvlr_assert!(
        pool_tokens_after == pool_tokens_before + amount
    );
}
