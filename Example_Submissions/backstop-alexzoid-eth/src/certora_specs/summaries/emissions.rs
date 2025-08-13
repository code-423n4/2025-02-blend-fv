#![allow(unused)]
use soroban_sdk::{Env, Address};
use cvlr::cvlr_assert;
use crate::BackstopError;
use soroban_sdk::panic_with_error;
use crate::certora_specs::base::GhostMap;
use crate::{PoolBalance, UserBalance};

pub(crate) static mut GHOST_EMISSION_POOL_BALANCE: GhostMap<PoolBalance> = GhostMap::UnInit;
pub(crate) static mut GHOST_EMISSION_USER_BALANCE: GhostMap<UserBalance> = GhostMap::UnInit;

pub fn update_emissions(
     e: &Env,
     pool_id: &Address,
     pool_balance: &PoolBalance,
     user_id: &Address,
     user_balance: &UserBalance,
) {
    unsafe {
        GHOST_EMISSION_POOL_BALANCE.init(pool_balance.clone());
        GHOST_EMISSION_USER_BALANCE.init(user_balance.clone());    
    }
}