use soroban_sdk::{
    contracttype, unwrap::UnwrapOptimized, vec, Address, Env, IntoVal, Symbol, TryFromVal, Val, Vec,
};

use crate::backstop::{PoolBalance, UserBalance};
use crate::certora_specs::GhostMap;

pub(crate) static mut balance_of_user: GhostMap<(Address, Address), UserBalance> = GhostMap::UnInit;

pub(crate) static mut balance_of_pool: GhostMap< Address, PoolBalance> = GhostMap::UnInit;

pub fn get_user_balance(e: &Env, pool: &Address, user: &Address) -> UserBalance {
    unsafe {
        return balance_of_user.get(&(pool.clone(),user.clone()));
    }
}

pub fn set_user_balance(e: &Env, pool: &Address, user: &Address, balance: &UserBalance) {
    unsafe{
        balance_of_user.set(&(pool.clone(),user.clone()), balance.clone());
    }
}

pub fn get_pool_balance(e: &Env, pool: &Address) -> PoolBalance {
    unsafe{
        return balance_of_pool.get(pool);
    }
}

pub fn set_pool_balance(e: &Env, pool: &Address, balance: &PoolBalance) {
    unsafe{
        balance_of_pool.set(pool, balance.clone());
    }
}

pub fn get_backstop_token(e: &Env) -> Address {
    return Address::from_str(e, "backstop");
}

pub fn get_pool_factory(e: &Env) -> Address {
    return Address::from_str(e, "pool_factory");
}

pub fn get_usdc_token(e: &Env) -> Address {
    return Address::from_str(e, "usdc");
}

pub fn get_blnd_token(e: &Env) -> Address {
    return Address::from_str(e, "blnd");
}

pub fn init_ghost(user: &Address,pool: &Address) {
    init_user_ghost(user, pool);
    let mut pool_balance: PoolBalance = cvlr::nondet();
    unsafe{
        balance_of_pool.init(pool, pool_balance);
    }
}

pub fn init_user_ghost(user: &Address,pool: &Address){
    let mut user_balance:UserBalance = cvlr::nondet();
    unsafe{
        balance_of_user.init(&(pool.clone(),user.clone()),user_balance);
    }
}
