#![allow(unused)]
use soroban_sdk::{Env, Address};
use crate::certora_specs::{GHOST_FROM_BALANCE, GHOST_TO_BALANCE, GHOST_ALLOWANCE}; 

use crate::{storage::BackstopEmissionData, PoolBalance, UserBalance};

pub fn transfer_mock(from: &Address, to: &Address, amount: &i128) {
    let from_balance = unsafe{GHOST_FROM_BALANCE};
    //panic if balance from is less than amount
    if from_balance < *amount {
        panic!("Insufficient balance to transfer");
    }
    unsafe{GHOST_FROM_BALANCE = from_balance - amount};
    let to_balance = unsafe{GHOST_TO_BALANCE};
    unsafe{GHOST_TO_BALANCE = to_balance + amount};
}

pub fn transfer_from_mock(spender: &Address, from: &Address, to: &Address, amount: &i128) {
    let allowance = unsafe{GHOST_ALLOWANCE};
    if allowance < *amount {
        panic!("Insufficient allowance to transfer");
    }

    transfer_mock(from, to, amount);
}
