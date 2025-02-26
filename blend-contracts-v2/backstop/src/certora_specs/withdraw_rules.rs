use cvlr::{cvlr_assert, cvlr_assume};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::backstop::{execute_queue_withdrawal, execute_withdraw};

// shares to withdraw must be nonnegative
#[rule]
pub fn amount_to_withdraw_nonnegative(e: &Env, from: Address, pool_address: Address, shares: i128) {
    // cvlr_assume!(shares < 0);
    execute_withdraw(e, &from, &pool_address, shares);
    cvlr_assert!(false); // should pass when assumption is enabled, fail otherwise
}

// withdraw queue entries are all positive
// needs -smt_preciseBitwiseOps
#[rule]
pub fn withdraw_queue_only_positive(e: Env, from: Address, pool_address: Address, amount: i128) {
    // cvlr_assume!(amount < 0);
    execute_queue_withdrawal(&e, &from, &pool_address, amount);
    cvlr_assert!(false); // should pass when assumption is enabled, fail otherwise
}
