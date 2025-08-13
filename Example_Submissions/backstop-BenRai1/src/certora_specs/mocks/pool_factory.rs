use soroban_sdk::{Address, BytesN, Env, String}; //@audit file created for ghost
use cvlr_soroban::nondet_address;

use crate::certora_specs::GHOST_IS_POOL;

pub struct PoolFactoryClient<'a> {
    pub env: Env,
    pub address: Address,
    _phantom: core::marker::PhantomData<&'a ()>,
}

pub trait PoolFactoryInterface {
    fn deploy(
        admin: Address,
        name: String,
        salt: BytesN<32>,
        oracle: Address,
        backstop_take_rate: u32,
        max_positions: u32,
        min_collateral: i128,
    ) -> Address;
    
    fn is_pool(pool_id: Address) -> bool;
}

impl<'a> PoolFactoryClient<'a> {
    pub fn new(env: &Env, address: &Address) -> Self {
        Self {
            env: env.clone(),
            address: address.clone(),
            _phantom: core::marker::PhantomData,
        }
    }
    
    pub fn deploy(
        &self,
        _e: Env,
        _admin: Address,
        _name: String,
        _salt: BytesN<32>,
        _oracle: Address,
        _backstop_take_rate: u32,
        _max_positions: u32,
        _min_collateral: i128,
    ) -> Address {
        nondet_address()
    }

    pub fn is_pool(&self, _pool_id: &Address) -> bool {
        let res = cvlr::nondet();
        unsafe {
            GHOST_IS_POOL = res;
        }
        return res
       
    }
}