use soroban_sdk::{Address, Env, String};
use cvlr_soroban_derive::cvlr_mock_client as mock_client;

use crate::certora_specs::{GHOST_FROM_BALANCE, GHOST_TO_BALANCE, GHOST_ALLOWANCE};



#[mock_client(name = "TokenClient")]
trait _TokenInterface {
    // fn allowance(env: Env, from: Address, spender: Address) -> i128;
    // fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32);
    // fn balance(env: Env, id: Address) -> i128;
    fn transfer(env: Env, from: Address, to: Address, amount: i128);
    fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128);
    // fn burn(env: Env, from: Address, amount: i128);
    // fn burn_from(env: Env, spender: Address, from: Address, amount: i128);
    // fn decimals(env: Env) -> u32;
    // fn name(env: Env) -> String;
    // fn symbol(env: Env) -> String;
}

struct Token {
    // balances: HashMap<Address, i128>,
    // allowances: HashMap<(Address, Address), i128>,
    total_supply: i128,
    name: String,
    symbol: String,
    decimals: u32,
}

impl _TokenInterface for Token {
    // fn balance(env: Env, id: Address) -> i128 {
    //     unsafe{GHOST_USER_BALANCE.get(&id)}
    // }

    fn transfer(_env: Env, _from: Address, _to: Address, amount: i128) {
        let from_balance = unsafe{GHOST_FROM_BALANCE};
        //panic if balance from is less than amount
        if from_balance < amount {
            panic!("Insufficient balance to transfer");
        }
        unsafe{GHOST_FROM_BALANCE = from_balance - amount};
        let to_balance = unsafe{GHOST_TO_BALANCE};
        unsafe{GHOST_TO_BALANCE = to_balance + amount};
    }

    // fn allowance(env: Env, from: Address, spender: Address) -> i128 {
    //     *self.allowances.get(&(from, spender)).unwrap_or(&0)
    // }

    // fn approve(env: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
    //     self.allowances.insert((from, spender), amount);
    //     // Emit approval event (if applicable)
    // }

    fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
        let allowance = unsafe{GHOST_ALLOWANCE};
        if allowance < amount {
            panic!("Insufficient allowance to transfer");
        }
        unsafe{GHOST_ALLOWANCE = allowance - amount};
        Self::transfer(env, from, to, amount);
    }

    // fn burn(env: Env, from: Address, amount: i128) {
    //     let from_balance = self.balance(env, from);
    //     assert!(from_balance >= amount, "Insufficient balance to burn");
    //     self.balances.insert(from, from_balance - amount);
    //     self.total_supply -= amount;
    //     // Emit burn event (if applicable)
    // }

    // fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
    //     let allowance = self.allowance(env, from, spender);
    //     assert!(allowance >= amount, "Allowance exceeded for burn");
    //     self.allowances.insert((from, spender), allowance - amount);
    //     self.burn(env, from, amount);
    // }

}

