use soroban_sdk::{Env, Address, unwrap::UnwrapOptimized};
use cvlr::{cvlr_assume, cvlr_assert};
use crate::backstop::{PoolBalance, UserBalance, execute_deposit, execute_withdraw, load_pool_backstop_data};
use cvlr_soroban_derive::rule;
use crate::init_verification;
use crate::certora_specs::FV_MAX_Q4W_VEC_LEN;
use crate::certora_specs::valid_state::valid_state_pool_user;
use crate::certora_specs::base::clear_upper_bits;
use crate::constants::SCALAR_7;
use soroban_fixed_point_math::FixedPoint;

#[cfg(feature = "certora_storage_ghost")] 
use crate::certora_specs::summaries::storage;
#[cfg(not(feature = "certora_storage_ghost"))]
use crate::storage;

// Verify that execute_deposit returns shares correctly converted from tokens
#[rule]
pub fn high_level_deposit_returns_converted_shares(
    e: &Env, 
    from: &Address, 
    pool_address: &Address, 
    amount: i128,
    pb: PoolBalance,
    ub: UserBalance
) {
    init_verification!(e, pb, ub, pool_address, from, amount, FV_MAX_Q4W_VEC_LEN);

    let before_pb: PoolBalance = storage::get_pool_balance(&e, pool_address);
        
    let expected_shares = before_pb.convert_to_shares(amount);
    
    let actual_shares = execute_deposit(e, from, pool_address, amount);
    
    // Verify that the returned shares match what we calculated
    cvlr_assert!(actual_shares == expected_shares);
}

// Checks that it's impossible to withdraw shares before the expiration time
#[rule]
pub fn high_level_withdrawal_expiration_enforced(
    e: &Env, 
    from: &Address, 
    pool_address: &Address, 
    amount: i128,
    pb: PoolBalance,
    ub: UserBalance
) {
    init_verification!(e, pb, ub, pool_address, from, amount, FV_MAX_Q4W_VEC_LEN);
    
    let ub_initial = storage::get_user_balance(&e, pool_address, from);
    cvlr_assume!(ub_initial.q4w.len() == 1);
    
    // Assume time not passed 
    let entry = ub_initial.q4w.get(0).unwrap_optimized();
    cvlr_assume!(e.ledger().timestamp() < entry.exp);
    
    execute_withdraw(e, from, pool_address, amount);
    
    // Should fail with NotExpired
    cvlr_assert!(false);
}

// When a pool is empty (zero shares), token-to-share conversion is 1:1 as expected for initial deposits
#[rule]
pub fn high_level_share_token_initial_conversion(
    e: &Env, 
    pool_address: &Address,
    tokens_amount: i128,
    pb: PoolBalance,
    ub: UserBalance
) {
    init_verification!(e, pb, ub, pool_address, pool_address, tokens_amount, FV_MAX_Q4W_VEC_LEN);
    
    let pool_balance = storage::get_pool_balance(&e, pool_address);
    cvlr_assume!(pool_balance.shares == 0);

    // Zero shares means 1:1 conversion (initial deposit case)
    let computed_shares = pool_balance.convert_to_shares(tokens_amount);
    cvlr_assert!(computed_shares == tokens_amount);
    
    let computed_tokens = pool_balance.convert_to_tokens(tokens_amount);
    cvlr_assert!(computed_tokens == tokens_amount);
}

// Token/share conversions maintain consistent exchange rates and roundtrip conversions
#[rule]
pub fn high_level_share_token_conversion(
    e: &Env, 
    pool_address: &Address,
    tokens_amount: i128,
    pb: PoolBalance,
    ub: UserBalance
) {
    init_verification!(e, pb, ub, pool_address, pool_address, tokens_amount, FV_MAX_Q4W_VEC_LEN);
    
    let pool_balance = storage::get_pool_balance(&e, pool_address);
    
    cvlr_assume!(pool_balance.shares != 0);
    cvlr_assume!(tokens_amount > 0);

    let computed_shares = pool_balance.convert_to_shares(tokens_amount);
    let roundtrip_tokens = pool_balance.convert_to_tokens(computed_shares);
    
    // Due to fixed-point math rounding, roundtrip might lose a small amount
    let diff = tokens_amount - roundtrip_tokens;
    cvlr_assert!(diff >= 0 && diff <= 2);
    
    // Shares/tokens ratio should be consistent
    if computed_shares > 0 {
        let input_ratio = tokens_amount.fixed_div_floor(computed_shares, SCALAR_7).unwrap_optimized();
        let pool_ratio = pool_balance.tokens.fixed_div_floor(pool_balance.shares, SCALAR_7).unwrap_optimized();
        
        // Allow for minimal rounding error (at most 1 per division)
        let ratio_diff = if input_ratio > pool_ratio { 
            input_ratio - pool_ratio 
        } else { 
            pool_ratio - input_ratio 
        };
        cvlr_assert!(ratio_diff <= 2);
    }

    cvlr_assert!(true);
}