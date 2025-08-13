use crate::backstop::{PoolBalance, UserBalance};
use crate::certora_specs::mocks;
use soroban_sdk::{Address, Env, unwrap::UnwrapOptimized};
use crate::constants::Q4W_LOCK_TIME;

#[cfg(feature = "certora_storage_ghost")] 
use crate::certora_specs::summaries::storage;
#[cfg(not(feature = "certora_storage_ghost"))]
use crate::storage;

// All valid state functions in one place
pub fn valid_state_pool_user(
    e: Env,
    pool: Address,
    user: Address
) -> bool {
    // valid_state_nonnegative
    valid_state_nonnegative_pb_shares(e.clone(), pool.clone(), user.clone())
    && valid_state_nonnegative_pb_tokens(e.clone(), pool.clone(), user.clone())
    && valid_state_nonnegative_pb_q4w(e.clone(), pool.clone(), user.clone())
    && valid_state_nonnegative_ub_shares(e.clone(), pool.clone(), user.clone())
    && valid_state_nonnegative_ub_q4w_amount(e.clone(), pool.clone(), user.clone())

    // valid_state_pb
    && valid_state_pb_q4w_leq_shares(e.clone(), pool.clone(), user.clone())

    // valid_state_ub
    && valid_state_ub_shares_plus_q4w_sum_eq_pb_shares(e.clone(), pool.clone(), user.clone())
    && valid_state_ub_q4w_sum_eq_pb_q4w(e.clone(), pool.clone(), user.clone())
    && valid_state_ub_q4w_expiration(e.clone(), pool.clone(), user.clone())
    && valid_state_ub_q4w_exp_implies_amount(e.clone(), pool.clone(), user.clone())

    // valid_state
    && valid_state_user_not_pool(e.clone(), pool.clone(), user.clone())
    && valid_state_pool_from_factory(e.clone(), pool.clone(), user.clone())
}

// PoolBalance shares are non-negative
pub fn valid_state_nonnegative_pb_shares(
    e: Env,
    pool: Address,
    _user: Address
) -> bool {
    let pb: PoolBalance = storage::get_pool_balance(&e, &pool);

    pb.shares.is_negative() == false 
}

// PoolBalance q4w and tokens are non-negative
pub fn valid_state_nonnegative_pb_q4w(
    e: Env,
    pool: Address,
    _user: Address
) -> bool {
    let pb: PoolBalance = storage::get_pool_balance(&e, &pool);

    pb.q4w.is_negative() == false
}

// PoolBalance tokens and tokens are non-negative
pub fn valid_state_nonnegative_pb_tokens(
    e: Env,
    pool: Address,
    _user: Address
) -> bool {
    let pb: PoolBalance = storage::get_pool_balance(&e, &pool);

    pb.tokens.is_negative() == false
}

// UserBalance shares are non-negative
pub fn valid_state_nonnegative_ub_shares(
    e: Env,
    pool: Address,
    user: Address
) -> bool {
    let ub: UserBalance = storage::get_user_balance(&e, &pool, &user);

    ub.shares.is_negative() == false
}

// UserBalance amount in q4w is non-negative
pub fn valid_state_nonnegative_ub_q4w_amount(
    e: Env,
    pool: Address,
    user: Address
) -> bool {
    let ub: UserBalance = storage::get_user_balance(&e, &pool, &user);

    if ub.q4w.len() != 0 {
        let entry0 = ub.q4w.get(0).unwrap_optimized();
        if entry0.amount.is_negative() {
            return false;
        }

        if ub.q4w.len() == 2 {
            let entry1 = ub.q4w.get(1).unwrap_optimized();
            if entry1.amount.is_negative() {
                return false;
            }            
        }     
    } 

    true
}

// q4w (shares queued for withdrawal) should never exceed total shares
pub fn valid_state_pb_q4w_leq_shares (
    e: Env,
    pool: Address,
    _user: Address
) -> bool {
    let pb: PoolBalance = storage::get_pool_balance(&e, &pool);
    
    pb.q4w <= pb.shares
}

// The expiration time (exp) in any Q4W entry must not exceed timestamp + Q4W_LOCK_TIME
pub fn valid_state_ub_q4w_expiration(
    e: Env,
    pool: Address,
    user: Address
) -> bool {
    let ub: UserBalance = storage::get_user_balance(&e, &pool, &user);

    if ub.q4w.len() != 0 {
        let max_timestamp = e.ledger().timestamp() + Q4W_LOCK_TIME;
        let entry0 = ub.q4w.get(0).unwrap_optimized();
        if entry0.exp > max_timestamp {
            return false;
        }

        if ub.q4w.len() == 2 {
            let entry1 = ub.q4w.get(1).unwrap_optimized();
            if entry1.exp > max_timestamp {
                return false;
            }
        }     
    } 

    true
}

// If a Q4W entry has a non-zero expiration time, it must have a non-zero amount
pub fn valid_state_ub_q4w_exp_implies_amount(
    e: Env,
    pool: Address,
    user: Address
) -> bool {
    let ub: UserBalance = storage::get_user_balance(&e, &pool, &user);

    if ub.q4w.len() != 0 {
        let entry0 = ub.q4w.get(0).unwrap_optimized();
        // If expiration is set (non-zero), amount must also be set (non-zero)
        if entry0.exp > 0 && entry0.amount == 0 {
            return false;
        }
    } 

    true
}

// A user's shares plus all q4w amounts equal to the total pool shares
// @note weird violations without i64 casting
pub fn valid_state_ub_shares_plus_q4w_sum_eq_pb_shares(
    e: Env,
    pool: Address,
    user: Address
) -> bool {
    let pb: PoolBalance = storage::get_pool_balance(&e, &pool);
    let ub: UserBalance = storage::get_user_balance(&e, &pool, &user);
    
    let mut sum = ub.shares as i64;
    if ub.q4w.len() != 0 {
        let entry0 = ub.q4w.get(0).unwrap_optimized();
        sum += entry0.amount as i64;

        if ub.q4w.len() == 2 {
            let entry1 = ub.q4w.get(1).unwrap_optimized();
            sum += entry1.amount as i64;
        } 
    } 

    sum == pb.shares as i64
}

// The sum of all amounts in the q4w vector must be less than pool's q4w
// @note weird violations without i64 casting
pub fn valid_state_ub_q4w_sum_eq_pb_q4w(
    e: Env,
    pool: Address,
    user: Address
) -> bool {
    let pb: PoolBalance = storage::get_pool_balance(&e, &pool);
    let ub: UserBalance = storage::get_user_balance(&e, &pool, &user);

    let mut q4w_sum = 0;
    if ub.q4w.len() != 0 {
        let entry0 = ub.q4w.get(0).unwrap_optimized();
        q4w_sum += entry0.amount as i64;

        if ub.q4w.len() == 2 {
            let entry1 = ub.q4w.get(1).unwrap_optimized();
            q4w_sum += entry1.amount as i64;   
        }    
    }

    pb.q4w as i64 == q4w_sum
}

// User who equals the pool address or the contract address always has a zero balance in that pool
pub fn valid_state_user_not_pool(
    e: Env, 
    pool: Address, 
    user: Address
) -> bool {
    let pb: PoolBalance = storage::get_pool_balance(&e, &pool);
    let ub: UserBalance = storage::get_user_balance(&e, &pool, &user);

    if user == pool || user == e.current_contract_address() {
        pb.shares == 0 && pb.tokens == 0 && pb.q4w == 0 && ub.shares == 0
    } else {
        true
    }
}

// Only deployed by the Pool Factory pool could have shares 
pub fn valid_state_pool_from_factory(
    e: Env, 
    pool: Address, 
    _user: Address
) -> bool {
    let pb: PoolBalance = storage::get_pool_balance(&e, &pool);
    let pool_factory_client = mocks::pool_factory::PoolFactoryClient::new(&e, &pool);
    
    if pool_factory_client.is_pool(&pool) == false {
        pb.shares == 0 && pb.tokens == 0 && pb.q4w == 0
    } else {
        true
    }
}