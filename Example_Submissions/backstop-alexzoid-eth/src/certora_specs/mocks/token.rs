use soroban_sdk::{Env, Address, String};
use cvlr_soroban_derive::cvlr_mock_client as mock_client;
use cvlr_soroban::nondet_string;
use crate::certora_specs::mocks::token_ghost;

#[mock_client(name = "TokenClientBase")]
trait _TokenInterface {
    fn allowance(env: Env, from: Address, spender: Address) -> i128;
    fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32);
    fn balance(env: Env, id: Address) -> i128;
    fn transfer(env: Env, from: Address, to: Address, amount: i128);
    fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128);
    fn burn(env: Env, from: Address, amount: i128);
    fn burn_from(env: Env, spender: Address, from: Address, amount: i128);
    fn decimals(env: Env) -> u32;
    fn name(env: Env) -> String;
    fn symbol(env: Env) -> String;
}

pub struct TokenClient<'a> {
    base: TokenClientBase<'a>,
}

impl<'a> TokenClient<'a> {

    pub fn new(env: &'a Env, address: &'a Address) -> Self {
        Self {
            base: TokenClientBase::new(env, address),
        }
    }

    pub fn transfer(&self, from: &Address, to: &Address, amount: &i128) {
        token_ghost::record_transfer(from, to, *amount);
        self.base.transfer(&from, &to, amount);
    }

    pub fn transfer_from(&self, spender: &Address, from: &Address, to: &Address, amount: &i128) {
        token_ghost::record_transfer_from(spender, from, to, *amount);
        self.base.transfer_from(spender, from, to, amount);
    }

    pub fn balance(&self, id: &Address) -> i128 {
        self.base.balance(id)
    }

    pub fn allowance(&self, from: &Address, spender: &Address) -> i128 {
        self.base.allowance(from, spender)
    }

    pub fn approve(&self, from: &Address, spender: &Address, amount: &i128, expiration_ledger: &u32) {
        self.base.approve(from, spender, amount, expiration_ledger)
    }

    pub fn burn(&self, from: &Address, amount: &i128) {
        self.base.burn(from, amount)
    }

    pub fn burn_from(&self, spender: &Address, from: &Address, amount: &i128) {
        self.base.burn_from(spender, from, amount)
    }

    pub fn decimals(&self) -> u32 {
        self.base.decimals()
    }

    pub fn name(&self) -> String {
        self.base.name()
    }

    pub fn symbol(&self) -> String {
        self.base.symbol()
    }
}