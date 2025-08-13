use soroban_sdk::{Env, Address, unwrap::UnwrapOptimized};
use cvlr::clog;
use crate::certora_specs::summaries::storage;

// Log an i128 value by splitting it into parts that fit within i64 range

pub fn log_input_amount(value: i128) {
    let input_amount_lower = (value.abs() & 0x7FFF_FFFF_FFFF_FFFF) as i64;
    let input_amount_upper = (value.abs() >> 63) as i64;
    let input_amount_neg = value.is_negative();
    
    clog!(input_amount_lower);
    clog!(input_amount_upper);
    clog!(input_amount_neg);
}

pub fn log_pb_shares(value: i128) {
    let pb_shares_lower = (value.abs() & 0x7FFF_FFFF_FFFF_FFFF) as i64;
    let pb_shares_upper = (value.abs() >> 63) as i64;
    let pb_shares_neg = value.is_negative();
    
    clog!(pb_shares_lower);
    clog!(pb_shares_upper);
    clog!(pb_shares_neg);
}

pub fn log_pb_tokens(value: i128) {
    let pb_tokens_lower = (value.abs() & 0x7FFF_FFFF_FFFF_FFFF) as i64;
    let pb_tokens_upper = (value.abs() >> 63) as i64;
    let pb_tokens_neg = value.is_negative();
    
    clog!(pb_tokens_lower);
    clog!(pb_tokens_upper);
    clog!(pb_tokens_neg);
}

pub fn log_pb_q4w(value: i128) {
    let pb_q4w_lower = (value.abs() & 0x7FFF_FFFF_FFFF_FFFF) as i64;
    let pb_q4w_upper = (value.abs() >> 63) as i64;
    let pb_q4w_neg = value.is_negative();
    
    clog!(pb_q4w_lower);
    clog!(pb_q4w_upper);
    clog!(pb_q4w_neg);
}

pub fn log_ub_shares(value: i128) {
    let ub_shares_lower = (value.abs() & 0x7FFF_FFFF_FFFF_FFFF) as i64;
    let ub_shares_upper = (value.abs() >> 63) as i64;
    let ub_shares_neg= value.is_negative();
    
    clog!(ub_shares_lower);
    clog!(ub_shares_upper);
    clog!(ub_shares_neg);
}

pub fn log_ub_q4w_amount(value: i128) {
    let ub_q4w_amount_lower = (value.abs() & 0x7FFF_FFFF_FFFF_FFFF) as i64;
    let ub_q4w_amount_upper = (value.abs() >> 63) as i64;
    let ub_q4w_amount_neg = value.is_negative();
    
    clog!(ub_q4w_amount_lower);
    clog!(ub_q4w_amount_upper);
    clog!(ub_q4w_amount_neg);
}

pub fn log_state_details(
    e: Env,
    pool: Address,
    user: Address,
    amount: i128
) {
    let pb = storage::get_pool_balance(&e, &pool);
    let ub = storage::get_user_balance(&e, &pool, &user);

    log_input_amount(amount);
    clog!(user == pool);
    clog!(user == e.current_contract_address());

    log_pb_shares(pb.shares);
    log_pb_tokens(pb.tokens);
    log_pb_q4w(pb.q4w);
    
    log_ub_shares(ub.shares);
    clog!(ub.q4w.len());
    
    if ub.q4w.len() != 0 {
        let entry0: crate::Q4W = ub.q4w.get(0).unwrap_optimized();
        log_ub_q4w_amount(entry0.amount);
        clog!(entry0.exp);
        if ub.q4w.len() == 2 {
            let entry1: crate::Q4W = ub.q4w.get(1).unwrap_optimized();
            log_ub_q4w_amount(entry1.amount);
            clog!(entry1.exp);    
        }
    }
}