use soroban_sdk::{Env, Address, unwrap::UnwrapOptimized};
use cvlr::{cvlr_assume, cvlr_assert};
use crate::backstop::{PoolBalance, UserBalance, 
    execute_deposit,
    execute_withdraw,
    execute_queue_withdrawal,
    execute_dequeue_withdrawal,
    execute_donate,
    execute_draw,
    load_pool_backstop_data
};
use cvlr_soroban_derive::rule;
use crate::init_verification;
use crate::certora_specs::valid_state::valid_state_pool_user;
use crate::certora_specs::base::clear_upper_bits;
use crate::certora_specs::FV_MAX_Q4W_VEC_LEN;

#[cfg(feature = "certora_storage_ghost")] 
use crate::certora_specs::summaries::storage;
#[cfg(not(feature = "certora_storage_ghost"))]
use crate::storage;

#[rule]
pub fn integrity_balance_deposit(
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
    let before_ub_q4w_amount = if before_ub.q4w.len() == 1 {
        let entry = before_ub.q4w.get(0).unwrap_optimized();
        entry.amount
    } else {
        0
    };

    let ret = execute_deposit(e, from, pool_address, amount);

    let after_pb: PoolBalance = storage::get_pool_balance(&e, pool_address);
    let after_ub: UserBalance = storage::get_user_balance(&e, pool_address, from);
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
    
    cvlr_assert!(ub_shares_change == ret);              // User shares should increase by returned amount (minted shares)
    cvlr_assert!(pb_shares_change == ub_shares_change); // Pool shares should increase by same amount
    cvlr_assert!(pb_tokens_change > 0);                 // Pool tokens should increase as tokens are deposited
    cvlr_assert!(pb_q4w_change == 0);                   // Pool Q4W should remain unchanged
    cvlr_assert!(ub_q4w_amount_change == 0);            // User's queued withdrawal amount should remain unchanged
}

#[rule]
pub fn integrity_balance_withdraw(
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
    let before_ub_q4w_amount = if before_ub.q4w.len() == 1 {
        let entry = before_ub.q4w.get(0).unwrap_optimized();
        entry.amount
    } else {
        0
    };

    let ret = execute_withdraw(e, from, pool_address, amount);

    let after_pb: PoolBalance = storage::get_pool_balance(&e, pool_address);
    let after_ub: UserBalance = storage::get_user_balance(&e, pool_address, from);
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
    
    cvlr_assert!(ub_q4w_amount_change == -amount);  // User Q4W amount should decrease by amount
    cvlr_assert!(pb_q4w_change == -amount);         // Pool Q4W should decrease by amount
    cvlr_assert!(pb_shares_change == -amount);      // Pool shares should decrease by amount
    cvlr_assert!(pb_tokens_change == -ret);         // Pool tokens should decrease by returned amount
    cvlr_assert!(ub_shares_change == 0);            // User shares should not change
}

#[rule]
pub fn integrity_balance_queue_withdrawal(
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
    let before_ub_q4w_amount = if before_ub.q4w.len() == 1 {
        let entry = before_ub.q4w.get(0).unwrap_optimized();
        entry.amount
    } else {
        0
    };

    let ret = execute_queue_withdrawal(e, from, pool_address, amount);

    let after_pb: PoolBalance = storage::get_pool_balance(&e, pool_address);
    let after_ub: UserBalance = storage::get_user_balance(&e, pool_address, from);
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
    
    cvlr_assert!(ub_shares_change == -amount);      // User shares should decrease by amount
    cvlr_assert!(ub_q4w_amount_change == amount);   // User Q4W amount should increase by amount
    cvlr_assert!(pb_q4w_change == amount);          // Pool Q4W should increase by amount
    cvlr_assert!(pb_shares_change == 0);            // Pool shares should not change
    cvlr_assert!(pb_tokens_change == 0);            // Pool tokens should not change
    cvlr_assert!(ret.amount == amount);             // Returned Q4W should have correct amount
    cvlr_assert!(ret.exp > e.ledger().timestamp()); // Expiration should be in the future
}

#[rule]
pub fn integrity_balance_dequeue_withdrawal(
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
    let before_ub_q4w_amount = if before_ub.q4w.len() == 1 {
        let entry = before_ub.q4w.get(0).unwrap_optimized();
        entry.amount
    } else {
        0
    };

    execute_dequeue_withdrawal(e, from, pool_address, amount);

    let after_pb: PoolBalance = storage::get_pool_balance(&e, pool_address);
    let after_ub: UserBalance = storage::get_user_balance(&e, pool_address, from);
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
    
    cvlr_assert!(ub_shares_change == amount);       // User shares should increase by amount
    cvlr_assert!(ub_q4w_amount_change == -amount);  // User Q4W amount should decrease by amount
    cvlr_assert!(pb_q4w_change == -amount);         // Pool Q4W should decrease by amount
    cvlr_assert!(pb_shares_change == 0);            // Pool shares should not change
    cvlr_assert!(pb_tokens_change == 0);            // Pool tokens should not change
}

#[rule]
pub fn integrity_balance_donate(
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
    let before_ub_q4w_amount = if before_ub.q4w.len() == 1 {
        let entry = before_ub.q4w.get(0).unwrap_optimized();
        entry.amount
    } else {
        0
    };

    execute_donate(e, from, pool_address, amount);

    let after_pb: PoolBalance = storage::get_pool_balance(&e, pool_address);
    let after_ub: UserBalance = storage::get_user_balance(&e, pool_address, from);
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

    cvlr_assert!(pb_tokens_change == amount);   // Pool tokens should increase by amount
    cvlr_assert!(pb_shares_change == 0);        // Pool shares should not change
    cvlr_assert!(pb_q4w_change == 0);           // Pool Q4W should not change
    cvlr_assert!(ub_shares_change == 0);        // User shares should not change
    cvlr_assert!(ub_q4w_amount_change == 0);    // User Q4W should not change
}

#[rule]
pub fn integrity_balance_draw(
    e: &Env, 
    pool_address: &Address, 
    amount: i128,
    to: &Address,
    pb: PoolBalance,
    ub: UserBalance
) {
    init_verification!(e, pb, ub, pool_address, to, amount, FV_MAX_Q4W_VEC_LEN);

    let before_pb: PoolBalance = storage::get_pool_balance(&e, pool_address);
    let before_ub: UserBalance = storage::get_user_balance(&e, pool_address, to);
    let before_ub_q4w_amount = if before_ub.q4w.len() == 1 {
        let entry = before_ub.q4w.get(0).unwrap_optimized();
        entry.amount
    } else {
        0
    };

    execute_draw(e, pool_address, amount, to);

    let after_pb: PoolBalance = storage::get_pool_balance(&e, pool_address);
    let after_ub: UserBalance = storage::get_user_balance(&e, pool_address, to);
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
    
    cvlr_assert!(pb_tokens_change == -amount);  // Pool tokens should decrease by amount
    cvlr_assert!(pb_shares_change == 0);        // Pool shares should not change
    cvlr_assert!(pb_q4w_change == 0);           // Pool Q4W should not change
    cvlr_assert!(ub_shares_change == 0);        // User shares should not change
    cvlr_assert!(ub_q4w_amount_change == 0);    // User Q4W should not change
}

#[rule]
pub fn integrity_balance_load_pool_backstop_data(
    e: &Env, 
    address: &Address,
    pb: PoolBalance,
    ub: UserBalance
) {
    init_verification!(e, pb, ub, address, address, 0, FV_MAX_Q4W_VEC_LEN);
        
    let before_pb: PoolBalance = storage::get_pool_balance(&e, address);
    let before_ub: UserBalance = storage::get_user_balance(&e, address, address);
        
    let pool_backstop_data = load_pool_backstop_data(e, address);
    
    let after_pb: PoolBalance = storage::get_pool_balance(&e, address);
    let after_ub: UserBalance = storage::get_user_balance(&e, address, address);
    
    // Verify that state hasn't changed 
    cvlr_assert!(before_pb.shares == after_pb.shares);
    cvlr_assert!(before_pb.tokens == after_pb.tokens);
    cvlr_assert!(before_pb.q4w == after_pb.q4w);
    cvlr_assert!(before_ub.shares == after_ub.shares);
        
    // Partially verify return values
    cvlr_assert!(before_pb.tokens == pool_backstop_data.tokens);
    if before_pb.shares > 0 {
        cvlr_assert!(pool_backstop_data.q4w_pct != 0);
    } else {
        cvlr_assert!(pool_backstop_data.q4w_pct == 0);
    }
}