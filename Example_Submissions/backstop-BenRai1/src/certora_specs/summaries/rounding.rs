#![allow(unused)]
use soroban_sdk::{Env, Address};

use crate::constants::SCALAR_7;
use crate::{storage::BackstopEmissionData, PoolBalance, UserBalance};
use crate::certora_specs::{GHOST_BLND_PER_TOKEN, GHOST_POOL_USDC_BALANCE, GHOST_USDC_PER_TOKEN};
use crate::certora_specs::GHOST_POOL_BLND_BALANCE;


pub fn fixed_div_ceil(x: i128, y: i128, denominator: i128) -> i128 {
    if(unsafe{x == GHOST_POOL_BLND_BALANCE}){
        unsafe{GHOST_BLND_PER_TOKEN += denominator};
    } else if(unsafe{x == GHOST_POOL_USDC_BALANCE}){
        unsafe{GHOST_USDC_PER_TOKEN += denominator};
    }
    let r = x * y;
    let initial_result = r/ denominator;
    //check if threre is no rest
    let final_result; 
    if initial_result * denominator == initial_result {
        final_result = initial_result;
    } else{
        final_result = initial_result +1;
    }

    final_result
}

pub fn fixed_div_floor(x: i128, y: i128, denominator: i128) -> i128 {
    let result = (x * denominator) / y;
    result
}

//@audit-issue add some ghost detection to know which ghost to change? ceiling +1

pub fn fixed_mul_floor(x: i128, y: i128, denominator: i128) -> i128 {
    let result = (x * y) / denominator;
    result
}

pub fn fixed_mul_ceil(x: i128, y: i128, denominator: i128) -> i128 {
    let r = x * denominator;
    let initial_result = r/ y;
    //check if threre is no rest
    let final_result; 
    if initial_result * y == initial_result {
        final_result = initial_result;
    } else{
        final_result = initial_result +1;
    }

    final_result
}




