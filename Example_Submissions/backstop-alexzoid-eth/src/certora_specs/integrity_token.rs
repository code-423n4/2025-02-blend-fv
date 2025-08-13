use soroban_sdk::{Env, Address, unwrap::UnwrapOptimized};
use cvlr::{cvlr_assume, cvlr_assert};
use crate::backstop::{PoolBalance, UserBalance, 
    execute_deposit,
    execute_withdraw,
    execute_queue_withdrawal,
    execute_dequeue_withdrawal,
    execute_donate,
    execute_draw
};
use cvlr_soroban_derive::rule;
use crate::init_verification;
use crate::certora_specs::base::clear_upper_bits;
use crate::certora_specs::FV_MAX_Q4W_VEC_LEN;
use crate::certora_specs::valid_state::valid_state_pool_user;
use crate::certora_specs::mocks::token_ghost;
use crate::certora_specs::mocks::token_ghost::TokenOpType;

#[cfg(feature = "certora_storage_ghost")] 
use crate::certora_specs::summaries::storage;
#[cfg(not(feature = "certora_storage_ghost"))]
use crate::storage;

#[rule]
pub fn integrity_token_deposit(
    e: &Env, 
    from: &Address, 
    pool_address: &Address, 
    amount: i128,
    pb: PoolBalance,
    ub: UserBalance
) {
    init_verification!(e, pb, ub, pool_address, from, amount, FV_MAX_Q4W_VEC_LEN);
    
    token_ghost::initialize_ghost_token_ops();
    cvlr_assume!(token_ghost::get_last_token_op().is_none());
    
    execute_deposit(e, from, pool_address, amount);
    
    let last_op = token_ghost::get_last_token_op();
    cvlr_assert!(last_op.is_some());
    
    // The operation should be a transfer from 'from' to the contract address with the correct amount
    let op = last_op.unwrap();
    cvlr_assert!(op.op_type == TokenOpType::Transfer);
    cvlr_assert!(op.from == *from);
    cvlr_assert!(op.to == e.current_contract_address());
    cvlr_assert!(op.amount == amount);
}

#[rule]
pub fn integrity_token_withdraw(
    e: &Env, 
    from: &Address, 
    pool_address: &Address, 
    amount: i128,
    pb: PoolBalance,
    ub: UserBalance
) {
    init_verification!(e, pb, ub, pool_address, from, amount, FV_MAX_Q4W_VEC_LEN);
    
    token_ghost::initialize_ghost_token_ops();
    cvlr_assume!(token_ghost::get_last_token_op().is_none());
    
    let to_return = execute_withdraw(e, from, pool_address, amount);
    
    // Get the last token operation after execution
    let last_op = token_ghost::get_last_token_op();
    cvlr_assert!(last_op.is_some());
    
    // The operation should be a transfer from the contract to 'from' with the returned amount
    let op = last_op.unwrap();
    cvlr_assert!(op.op_type == TokenOpType::Transfer);
    cvlr_assert!(op.from == e.current_contract_address());
    cvlr_assert!(op.to == *from);
    cvlr_assert!(op.amount == to_return);
}

#[rule]
pub fn integrity_token_donate(
    e: &Env, 
    from: &Address, 
    pool_address: &Address, 
    amount: i128,
    pb: PoolBalance,
    ub: UserBalance
) {
    init_verification!(e, pb, ub, pool_address, from, amount, FV_MAX_Q4W_VEC_LEN);
    
    token_ghost::initialize_ghost_token_ops();
    cvlr_assume!(token_ghost::get_last_token_op().is_none());
    
    execute_donate(e, from, pool_address, amount);
    
    // Get the last token operation after execution
    let last_op = token_ghost::get_last_token_op();
    cvlr_assert!(last_op.is_some());
    
    // The operation should be a transfer_from with the contract as spender
    let op = last_op.unwrap();
    cvlr_assert!(op.op_type == TokenOpType::TransferFrom);
    cvlr_assert!(op.from == *from);
    cvlr_assert!(op.to == e.current_contract_address());
    cvlr_assert!(op.amount == amount);
}

#[rule]
pub fn integrity_token_draw(
    e: &Env, 
    pool_address: &Address, 
    amount: i128,
    to: &Address,
    pb: PoolBalance,
    ub: UserBalance
) {
    init_verification!(e, pb, ub, pool_address, to, amount, FV_MAX_Q4W_VEC_LEN);
    
    // Initialize token operations ghost state
    token_ghost::initialize_ghost_token_ops();
    cvlr_assume!(token_ghost::get_last_token_op().is_none());
    
    execute_draw(e, pool_address, amount, to);
    
    // Get the last token operation after execution
    let last_op = token_ghost::get_last_token_op();
    cvlr_assert!(last_op.is_some());
    
    // The operation should be a transfer from the contract to 'to' with the correct amount
    let op = last_op.unwrap();
    cvlr_assert!(op.op_type == TokenOpType::Transfer);
    cvlr_assert!(op.from == e.current_contract_address());
    cvlr_assert!(op.to == *to);
    cvlr_assert!(op.amount == amount);
}

#[rule]
pub fn integrity_token_queue_withdrawal(
    e: &Env, 
    from: &Address, 
    pool_address: &Address, 
    amount: i128,
    pb: PoolBalance,
    ub: UserBalance
) {
    init_verification!(e, pb, ub, pool_address, from, amount, FV_MAX_Q4W_VEC_LEN);
    
    token_ghost::initialize_ghost_token_ops();
    cvlr_assume!(token_ghost::get_last_token_op().is_none());
    
    execute_queue_withdrawal(e, from, pool_address, amount);
    
    // Verify no token operation occurred
    cvlr_assert!(token_ghost::get_last_token_op().is_none());
}

// Verify that dequeue_withdrawal does not transfer any tokens
#[rule]
pub fn integrity_token_dequeue_withdrawal(
    e: &Env, 
    from: &Address, 
    pool_address: &Address, 
    amount: i128,
    pb: PoolBalance,
    ub: UserBalance
) {
    init_verification!(e, pb, ub, pool_address, from, amount, FV_MAX_Q4W_VEC_LEN);
    
    token_ghost::initialize_ghost_token_ops();
    cvlr_assume!(token_ghost::get_last_token_op().is_none());
    
    execute_dequeue_withdrawal(e, from, pool_address, amount);
    
    // Verify no token operation occurred
    cvlr_assert!(token_ghost::get_last_token_op().is_none());
}