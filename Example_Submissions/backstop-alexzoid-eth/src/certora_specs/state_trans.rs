use soroban_sdk::{Env, Address, unwrap::UnwrapOptimized};
use cvlr::cvlr_assert;
use crate::backstop::{PoolBalance, UserBalance};

#[cfg(feature = "certora_storage_ghost")] 
use crate::certora_specs::summaries::storage;
#[cfg(not(feature = "certora_storage_ghost"))]
use crate::storage;

// If shares increase, tokens must increase and if shares decrease, tokens must decrease
pub fn state_trans_pb_shares_tokens_directional_change(
    e: &Env, 
    pool: &Address, 
    _user: &Address, 
    _amount: i128,
    call_fn: impl FnOnce()
) {
    let before: PoolBalance = storage::get_pool_balance(&e, pool);
    call_fn();
    let after: PoolBalance = storage::get_pool_balance(&e, pool);

    // If shares increase, tokens must increase
    if after.shares > before.shares {
        cvlr_assert!(after.tokens > before.tokens);
    }
    // If shares decrease, tokens must decrease
    else if after.shares < before.shares {
        cvlr_assert!(after.tokens < before.tokens);
    } 

    cvlr_assert!(true);
}

// Verify the relationship for pool balance q4w changes
pub fn state_trans_pb_q4w_consistency(
    e: &Env, 
    pool: &Address, 
    user: &Address, 
    _amount: i128,
    call_fn: impl FnOnce()
) {
    let before_pb: PoolBalance = storage::get_pool_balance(&e, pool);
    let before_ub: UserBalance = storage::get_user_balance(&e, pool, user);
    
    let before_ub_q4w_amount: i128 = if before_ub.q4w.len() == 1 {
        let entry = before_ub.q4w.get(0).unwrap_optimized();
        entry.amount
    } else {
        0
    };
    
    call_fn();
    
    let after_pb: PoolBalance = storage::get_pool_balance(&e, pool);
    let after_ub: UserBalance = storage::get_user_balance(&e, pool, user);
    
    let after_ub_q4w_amount: i128 = if after_ub.q4w.len() == 1 {
        let entry = after_ub.q4w.get(0).unwrap_optimized();
        entry.amount
    } else {
        0
    };
    
    let pb_q4w_change = after_pb.q4w - before_pb.q4w;
    let ub_q4w_amount_change = after_ub_q4w_amount - before_ub_q4w_amount;
    let ub_shares_change = after_ub.shares - before_ub.shares;
    let pb_shares_change = after_pb.shares - before_pb.shares;
    let pb_tokens_change = after_pb.tokens - before_pb.tokens;
    
    // Case 1: Queue for withdrawal
    if pb_q4w_change > 0 {
        cvlr_assert!(ub_shares_change == -pb_q4w_change);
        cvlr_assert!(ub_q4w_amount_change == pb_q4w_change);
        cvlr_assert!(pb_shares_change == 0); 
        cvlr_assert!(pb_tokens_change == 0); 
    }
    
    // Case 2: Withdraw (both pool q4w and shares decrease)
    if pb_q4w_change < 0 && pb_shares_change < 0 {
        cvlr_assert!(pb_shares_change == pb_q4w_change);
        cvlr_assert!(ub_q4w_amount_change == pb_q4w_change);
        cvlr_assert!(ub_shares_change == 0); 
    }
    
    // Case 3: Dequeue (q4w decreases, user shares increase)
    if pb_q4w_change < 0 && ub_shares_change > 0 {
        cvlr_assert!(ub_shares_change == -pb_q4w_change);
        cvlr_assert!(ub_q4w_amount_change == pb_q4w_change);
        cvlr_assert!(pb_shares_change == 0); 
        cvlr_assert!(pb_tokens_change == 0); 
    }

    cvlr_assert!(true);
}

// Verify balance relationships when user shares increase
pub fn state_trans_ub_shares_increase_consistency(
    e: &Env, 
    pool: &Address, 
    user: &Address, 
    _amount: i128,
    call_fn: impl FnOnce()
) {
    let before_pb: PoolBalance = storage::get_pool_balance(&e, pool);
    let before_ub: UserBalance = storage::get_user_balance(&e, pool, user);
    
    let before_ub_q4w_amount = if before_ub.q4w.len() == 1 {
        let entry = before_ub.q4w.get(0).unwrap_optimized();
        entry.amount
    } else {
        0
    };
    
    call_fn();
    
    let after_pb: PoolBalance = storage::get_pool_balance(&e, pool);
    let after_ub: UserBalance = storage::get_user_balance(&e, pool, user);
    
    let after_ub_q4w_amount = if after_ub.q4w.len() == 1 {
        let entry = after_ub.q4w.get(0).unwrap_optimized();
        entry.amount
    } else {
        0
    };
    
    let ub_shares_change = after_ub.shares - before_ub.shares;
    let pb_shares_change = after_pb.shares - before_pb.shares;
    let pb_tokens_change = after_pb.tokens - before_pb.tokens;
    let pb_q4w_change = after_pb.q4w - before_pb.q4w;
    let ub_q4w_amount_change = after_ub_q4w_amount - before_ub_q4w_amount;
    
    // User shares increase (deposit or dequeue)
    if ub_shares_change > 0 {
        // For deposit: pb.shares increase by same amount, tokens increase
        if pb_tokens_change > 0 {
            cvlr_assert!(pb_shares_change == ub_shares_change);
            cvlr_assert!(pb_tokens_change > 0);
            cvlr_assert!(pb_q4w_change == 0);
            cvlr_assert!(ub_q4w_amount_change == 0); 
        }
        
        // For dequeue_withdrawal: ub.q4w amount decreases by same amount
        else if ub_q4w_amount_change < 0 {
            cvlr_assert!(ub_q4w_amount_change == -ub_shares_change);
            cvlr_assert!(pb_q4w_change == ub_q4w_amount_change); 
            cvlr_assert!(pb_shares_change == 0); 
            cvlr_assert!(pb_tokens_change == 0);
        }
    }
        
    cvlr_assert!(true);
}

// Verify balance relationships when user shares change
pub fn state_trans_ub_shares_decrease_consistency(
    e: &Env, 
    pool: &Address, 
    user: &Address, 
    _amount: i128,
    call_fn: impl FnOnce()
) {
    let before_pb: PoolBalance = storage::get_pool_balance(&e, pool);
    let before_ub: UserBalance = storage::get_user_balance(&e, pool, user);
    
    let before_ub_q4w_amount = if before_ub.q4w.len() == 1 {
        let entry = before_ub.q4w.get(0).unwrap_optimized();
        entry.amount
    } else {
        0
    };
    
    call_fn();
    
    let after_pb: PoolBalance = storage::get_pool_balance(&e, pool);
    let after_ub: UserBalance = storage::get_user_balance(&e, pool, user);
    
    let after_ub_q4w_amount = if after_ub.q4w.len() == 1 {
        let entry = after_ub.q4w.get(0).unwrap_optimized();
        entry.amount
    } else {
        0
    };
    
    let ub_shares_change = after_ub.shares - before_ub.shares;
    let pb_shares_change = after_pb.shares - before_pb.shares;
    let pb_tokens_change = after_pb.tokens - before_pb.tokens;
    let pb_q4w_change = after_pb.q4w - before_pb.q4w;
    let ub_q4w_amount_change = after_ub_q4w_amount - before_ub_q4w_amount;
    
    // User shares decrease (queue_withdrawal)
    if ub_shares_change < 0 {
        // For queue_withdrawal: ub.q4w amount increases by same amount
        if ub_q4w_amount_change > 0 {
            cvlr_assert!(ub_q4w_amount_change == -ub_shares_change);
            cvlr_assert!(pb_q4w_change == ub_q4w_amount_change); 
            cvlr_assert!(pb_shares_change == 0); 
            cvlr_assert!(pb_tokens_change == 0); 
        }
    }
    
    cvlr_assert!(true);
}

// Verify balance relationships when user's queued withdrawal amount changes
pub fn state_trans_ub_q4w_amount_consistency(
    e: &Env, 
    pool: &Address, 
    user: &Address, 
    _amount: i128,
    call_fn: impl FnOnce()
) {
    let before_pb: PoolBalance = storage::get_pool_balance(&e, pool);
    let before_ub: UserBalance = storage::get_user_balance(&e, pool, user);
    
    let before_ub_q4w_amount = if before_ub.q4w.len() == 1 {
        let entry = before_ub.q4w.get(0).unwrap_optimized();
        entry.amount
    } else {
        0
    };
    
    call_fn();
    
    let after_pb: PoolBalance = storage::get_pool_balance(&e, pool);
    let after_ub: UserBalance = storage::get_user_balance(&e, pool, user);
    
    let after_ub_q4w_amount = if after_ub.q4w.len() == 1 {
        let entry = after_ub.q4w.get(0).unwrap_optimized();
        entry.amount
    } else {
        0
    };
    
    let ub_q4w_amount_change = after_ub_q4w_amount - before_ub_q4w_amount;
    let ub_shares_change = after_ub.shares - before_ub.shares;
    let pb_q4w_change = after_pb.q4w - before_pb.q4w;
    let pb_shares_change = after_pb.shares - before_pb.shares;
    let pb_tokens_change = after_pb.tokens - before_pb.tokens;

    // Case 1: User Q4W amount increases (queue_withdrawal)
    if ub_q4w_amount_change > 0 {
        cvlr_assert!(ub_shares_change == -ub_q4w_amount_change); 
        cvlr_assert!(pb_q4w_change == ub_q4w_amount_change); 
        cvlr_assert!(pb_shares_change == 0); 
        cvlr_assert!(pb_tokens_change == 0); 
    }
    
    // Case 2: User Q4W amount decreases (withdraw or dequeue)
    else if ub_q4w_amount_change < 0 {
        cvlr_assert!(pb_q4w_change == ub_q4w_amount_change); 
        
        // Case 2A: Withdraw operation (pool tokens and shares decrease)
        if pb_tokens_change < 0 {
            cvlr_assert!(pb_shares_change == ub_q4w_amount_change); 
            cvlr_assert!(pb_tokens_change < 0);
            cvlr_assert!(ub_shares_change == 0); 
        }
        
        // Case 2B: Dequeue operation (user shares increase)
        else if ub_shares_change > 0 {
            cvlr_assert!(ub_shares_change == -ub_q4w_amount_change); 
            cvlr_assert!(pb_shares_change == 0); 
            cvlr_assert!(pb_tokens_change == 0);
        }
    }

    cvlr_assert!(true);
}
