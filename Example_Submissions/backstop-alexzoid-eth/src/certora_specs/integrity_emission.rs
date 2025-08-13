use soroban_sdk::{Env, Address, unwrap::UnwrapOptimized};
use cvlr::{cvlr_assume, cvlr_assert};
use crate::backstop::{PoolBalance, UserBalance};
use cvlr_soroban_derive::rule;
use crate::init_verification;
use crate::certora_specs::valid_state::valid_state_pool_user;
use crate::certora_specs::FV_MAX_Q4W_VEC_LEN;
use crate::certora_specs::mocks::token_ghost;
use crate::certora_specs::base::clear_upper_bits;
use crate::certora_specs::summaries::emissions::{
    GHOST_EMISSION_POOL_BALANCE, 
    GHOST_EMISSION_USER_BALANCE,
};

#[cfg(feature = "certora_storage_ghost")] 
use crate::certora_specs::summaries::storage;
#[cfg(not(feature = "certora_storage_ghost"))]
use crate::storage;

fn verify_emission_state(
    before_pb: &PoolBalance,
    before_ub: &UserBalance
) {
    let emission_pb = unsafe { GHOST_EMISSION_POOL_BALANCE.get() };
    let emission_ub = unsafe { GHOST_EMISSION_USER_BALANCE.get() };
    
    cvlr_assert!(emission_pb.shares == before_pb.shares);
    cvlr_assert!(emission_pb.tokens == before_pb.tokens);
    cvlr_assert!(emission_pb.q4w == before_pb.q4w);
    cvlr_assert!(emission_ub.shares == before_ub.shares);
}

#[rule]
pub fn integrity_emission_deposit(
    e: &Env, 
    from: &Address, 
    pool_address: &Address, 
    amount: i128,
    pb: PoolBalance,
    ub: UserBalance
) {
    init_verification!(e, pb, ub, pool_address, from, amount, FV_MAX_Q4W_VEC_LEN);

    let before_pb: PoolBalance = storage::get_pool_balance(&e, pool_address);
    let before_ub: UserBalance = storage::get_user_balance(&e, pool_address, from);
        
    crate::backstop::execute_deposit(e, from, pool_address, amount);
    
    verify_emission_state(&before_pb, &before_ub);
}

#[rule]
pub fn integrity_emission_withdraw(
    e: &Env, 
    from: &Address, 
    pool_address: &Address, 
    amount: i128,
    pb: PoolBalance,
    ub: UserBalance
) {
    init_verification!(e, pb, ub, pool_address, from, amount, FV_MAX_Q4W_VEC_LEN);

    cvlr_assume!(unsafe { GHOST_EMISSION_POOL_BALANCE.is_uninit() });
    cvlr_assume!(unsafe { GHOST_EMISSION_USER_BALANCE.is_uninit() });

    crate::backstop::execute_withdraw(e, from, pool_address, amount);
    
    cvlr_assert!(unsafe { GHOST_EMISSION_POOL_BALANCE.is_uninit() });
    cvlr_assert!(unsafe { GHOST_EMISSION_USER_BALANCE.is_uninit() });
}

#[rule]
pub fn integrity_emission_queue_withdrawal(
    e: &Env, 
    from: &Address, 
    pool_address: &Address, 
    amount: i128,
    pb: PoolBalance,
    ub: UserBalance
) {
    init_verification!(e, pb, ub, pool_address, from, amount, FV_MAX_Q4W_VEC_LEN);

    let before_pb: PoolBalance = storage::get_pool_balance(&e, pool_address);
    let before_ub: UserBalance = storage::get_user_balance(&e, pool_address, from);
        
    crate::backstop::execute_queue_withdrawal(e, from, pool_address, amount);
    
    verify_emission_state(&before_pb, &before_ub);
}

#[rule]
pub fn integrity_emission_dequeue_withdrawal(
    e: &Env, 
    from: &Address, 
    pool_address: &Address, 
    amount: i128,
    pb: PoolBalance,
    ub: UserBalance
) {
    init_verification!(e, pb, ub, pool_address, from, amount, FV_MAX_Q4W_VEC_LEN);

    let before_pb: PoolBalance = storage::get_pool_balance(&e, pool_address);
    let before_ub: UserBalance = storage::get_user_balance(&e, pool_address, from);
        
    crate::backstop::execute_dequeue_withdrawal(e, from, pool_address, amount);
    
    verify_emission_state(&before_pb, &before_ub);
}

#[rule]
pub fn integrity_emission_donate(
    e: &Env, 
    from: &Address, 
    pool_address: &Address, 
    amount: i128,
    pb: PoolBalance,
    ub: UserBalance
) {
    init_verification!(e, pb, ub, pool_address, from, amount, FV_MAX_Q4W_VEC_LEN);

    cvlr_assume!(unsafe { GHOST_EMISSION_POOL_BALANCE.is_uninit() });
    cvlr_assume!(unsafe { GHOST_EMISSION_USER_BALANCE.is_uninit() });

    crate::backstop::execute_donate(e, from, pool_address, amount);
    
    cvlr_assert!(unsafe { GHOST_EMISSION_POOL_BALANCE.is_uninit() });
    cvlr_assert!(unsafe { GHOST_EMISSION_USER_BALANCE.is_uninit() });
}

#[rule]
pub fn integrity_emission_draw(
    e: &Env, 
    pool_address: &Address, 
    amount: i128,
    to: &Address,
    pb: PoolBalance,
    ub: UserBalance
) {
    init_verification!(e, pb, ub, pool_address, to, amount, FV_MAX_Q4W_VEC_LEN);

    cvlr_assume!(unsafe { GHOST_EMISSION_POOL_BALANCE.is_uninit() });
    cvlr_assume!(unsafe { GHOST_EMISSION_USER_BALANCE.is_uninit() });

    crate::backstop::execute_draw(e, pool_address, amount, to);
    
    cvlr_assert!(unsafe { GHOST_EMISSION_POOL_BALANCE.is_uninit() });
    cvlr_assert!(unsafe { GHOST_EMISSION_USER_BALANCE.is_uninit() });
}