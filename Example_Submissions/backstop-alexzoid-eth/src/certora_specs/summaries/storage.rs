use crate::backstop::{PoolBalance, UserBalance};
use cvlr_soroban::nondet_address;
use soroban_sdk::{Address, Env};
use crate::certora_specs::base::GhostMap;

pub(crate) static mut GHOST_POOL_BALANCE: GhostMap<PoolBalance> = GhostMap::UnInit;
pub(crate) static mut GHOST_USER_BALANCE: GhostMap<UserBalance> = GhostMap::UnInit;

pub fn get_user_balance(_e: &Env, _pool: &Address, _user: &Address) -> UserBalance {
    unsafe {
        GHOST_USER_BALANCE.get()
    }
}

pub fn set_user_balance(_e: &Env, _pool: &Address, _user: &Address, balance: &UserBalance) {
    unsafe {
        GHOST_USER_BALANCE.set(balance.clone());
    }
}

pub fn get_pool_balance(_e: &Env, _pool: &Address) -> PoolBalance {
    unsafe {
        GHOST_POOL_BALANCE.get()
    }
}

pub fn set_pool_balance(_e: &Env, _pool: &Address, balance: &PoolBalance) {
    unsafe {
        GHOST_POOL_BALANCE.set(balance.clone());
    }
}

pub fn initialize_ghost_maps(
    pool_balance: PoolBalance, 
    user_balance: UserBalance
) {
    unsafe {
        GHOST_POOL_BALANCE.init(pool_balance);
        GHOST_USER_BALANCE.init(user_balance);
    }
}

// Add additional required functions for the file-specific mocks
#[allow(dead_code)]
pub fn get_backstop_token(_e: &Env) -> Address {
    nondet_address()
}

#[allow(dead_code)]
pub fn get_blnd_token(_e: &Env) -> Address {
    nondet_address()
}

#[allow(dead_code)]
pub fn get_usdc_token(_e: &Env) -> Address {
    nondet_address()
}

#[allow(dead_code)]
pub fn get_emitter(_e: &Env) -> Address {
    nondet_address()
}