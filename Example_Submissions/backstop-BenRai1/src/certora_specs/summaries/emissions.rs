#![allow(unused)]
use soroban_sdk::{Env, Address};
use crate::certora_specs::GHOST_UPDATE_EMISSIONS_CALLED; 

use crate::{storage::BackstopEmissionData, PoolBalance, UserBalance};

pub fn update_emissions_mock(
    e: &Env,
    pool_id: &Address,
    pool_balance: &PoolBalance,
    user_id: &Address,
    user_balance: &UserBalance,
) {
    unsafe {
            GHOST_UPDATE_EMISSIONS_CALLED = true;
        }
}

//how to create ghost variables

//how to call the ghost variables