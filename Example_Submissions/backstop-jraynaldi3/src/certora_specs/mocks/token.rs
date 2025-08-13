use soroban_sdk::{Env, Address, String};
use cvlr_soroban_derive::cvlr_mock_client as mock_client;
use cvlr_soroban::nondet_string;
use cvlr::{cvlr_hook_on_entry, clog, cvlr_assume};
use crate::certora_specs::GhostMap;

#[mock_client(name = "TokenClient")]
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

pub struct TokenWrapper<'a> {
    pub env: Env,
    pub address: Address,
    _phantom: core::marker::PhantomData<&'a ()>
}

pub(crate) static mut balances: GhostMap<Address, i128> = GhostMap::UnInit;
pub(crate) static mut allowance: GhostMap<(Address, Address), i128> = GhostMap::UnInit;

impl<'a> TokenWrapper<'a> {
    pub fn new(env: &Env, address: &Address) -> Self {
        Self {
            env: env.clone(),
            address: address.clone(),
            _phantom: core::marker::PhantomData,
        }
    }

    pub fn transfer(&self, from: &Address, to: &Address, amount: &i128) {
        unsafe {
            let mut to_balance = balances.get(to);
            to_balance = to_balance + amount.clone();
            balances.set(to,to_balance);

            let mut from_balance = balances.get(from);
            from_balance = from_balance - amount.clone();
            balances.set(from,from_balance);
        }
    }

    pub fn transfer_from(&self, spender: &Address, from: &Address, to: &Address, amount: &i128) {
        unsafe {
            let mut spender_allowance = self.allowance(from, spender);
            cvlr_assume!(spender_allowance >= amount.clone());
            spender_allowance = spender_allowance - amount;
            allowance.set(&(from.clone(), spender.clone()),spender_allowance);
        }
        self.transfer(from, to, amount);
    }

    pub fn balance(&self, id: &Address) -> i128 {
        unsafe {
            return balances.get(id);
        }
    }

    pub fn allowance(&self, id: &Address, spender: &Address) -> i128 {
        unsafe {
            return allowance.get(&(id.clone(), spender.clone()));
        }
    }

    pub fn init_balance(&self, id: &Address) {
        let num :i128 = cvlr::nondet();
        clog!(num.clamp(i64::MIN as i128, i64::MAX as i128) as i64);
        unsafe{
            balances.init(id, num);
        }
    }    
    
    pub fn init_allowance(&self, id: &Address, spender: &Address) {
        let num :i128 = cvlr::nondet();
        unsafe{
            allowance.init(&(id.clone(), spender.clone()), num);
        }
    }
}

