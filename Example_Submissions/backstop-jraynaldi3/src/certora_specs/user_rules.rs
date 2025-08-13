use cvlr::{cvlr_assert, cvlr_assume, clog};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use crate::Q4W;
use crate::backstop::UserBalance;
use crate::constants::{MAX_Q4W_SIZE, Q4W_LOCK_TIME};

// deposit should increase user shares
#[rule]
pub fn add_shares_increases_user_shares(user_balance: &mut UserBalance, shares: i128) {
    cvlr_assume!(shares >= 0);
    let user_shares_before = user_balance.shares;
    user_balance.add_shares(shares);
    let user_shares_after = user_balance.shares;
    cvlr_assert!(user_shares_after == user_shares_before + shares);
}

/* -------------------------------------------------------------------------- */
/*                         queue_shares_for_withdrawal                        */
/* -------------------------------------------------------------------------- */
/// queue_shares_for_withdrawal cannot more than user_balance.shares
#[rule]
pub fn queue_shares_for_withdrawal_lte_balance(user_balance: &mut UserBalance, e: &Env, to_queue: i128) {
    let shares_before = user_balance.shares;
    user_balance.queue_shares_for_withdrawal(e, to_queue);
    cvlr_assert!(shares_before >= to_queue);
}

/// queue_shares_for_withdrawal q4w length should less tahan maximum
#[rule]
pub fn queue_shares_for_withdrawal_q4w_len_maximum(user_balance: &mut UserBalance, e: &Env, to_queue: i128) {
    user_balance.queue_shares_for_withdrawal(e, to_queue);
    cvlr_assert!(MAX_Q4W_SIZE >= user_balance.q4w.len());
}

/// queue_shares_for_withdrawal decerease amount shares
#[rule]
pub fn queue_shares_for_withdrawal_decrease_shares(e: &Env, to_queue: i128) {
    let mut user_balance:UserBalance = cvlr::nondet();
    let user_shares_before = user_balance.shares;
    cvlr_assume!(to_queue > 0 && user_shares_before > 0);

    user_balance.queue_shares_for_withdrawal(e, to_queue);

    let user_shares_after = user_balance.shares;
    cvlr_assert!(user_shares_after == user_shares_before - to_queue);
}

/// queue_shares_for_withdrawal should push q4w 
#[rule]
pub fn queue_shares_for_withdrawal_push_q4w(user_balance: &mut UserBalance, e: &Env, to_queue: i128) {
    user_balance.queue_shares_for_withdrawal(e, to_queue);
    let latest_queue = user_balance.q4w.last().unwrap();
    cvlr_assert!(
        latest_queue.amount == to_queue
        && latest_queue.exp == e.ledger().timestamp() + Q4W_LOCK_TIME
    );
}

/* -------------------------------------------------------------------------- */
/*                               withdraw_shares                              */
/* -------------------------------------------------------------------------- */

/// withdraw_shares should decrease first q4w amount 
#[rule]
pub fn withdraw_shares_first_q4w_amount_decrease( e: &Env, to_withdraw: i128) {

    let mut user_balance:UserBalance = cvlr::nondet();
    let first_q4w_amount_before = user_balance.q4w.first().unwrap().amount;
    cvlr_assume!(first_q4w_amount_before >= 0 && to_withdraw >= 0);
    cvlr_assume!(user_balance.q4w.len() == 2);

    user_balance.withdraw_shares(e, to_withdraw);

    let first_q4w_amount_after = user_balance.q4w.first().unwrap().amount;

    if first_q4w_amount_before > to_withdraw {
        cvlr_assert!(first_q4w_amount_after == first_q4w_amount_before - to_withdraw);
    } 
    // cvlr_assert!(true);
}

/// withdraw_shares should pop first q4w if amount is 0; 
#[rule]
pub fn withdraw_shares_first_q4w_pop(e: &Env, to_withdraw: i128) {

    let mut user_balance:UserBalance = cvlr::nondet();
    let first_q4w_amount_before = user_balance.q4w.first_unchecked().amount;
    let q4w_len_before = user_balance.q4w.len();
    cvlr_assume!(first_q4w_amount_before < to_withdraw); 
    cvlr_assume!(user_balance.q4w.len() == 2);

    user_balance.withdraw_shares(e, to_withdraw);

    let q4w_len_after = user_balance.q4w.len();

    cvlr_assert!(q4w_len_before > q4w_len_after);
}

/// withdraw_shares to_withdraw should less than total q4w amount; 
#[rule]
pub fn withdraw_shares_lte_q4w_amount( e: &Env, to_withdraw: i128) {

    let mut user_balance:UserBalance = cvlr::nondet();
    cvlr_assume!(user_balance.q4w.len() == 2);
    cvlr_assume!(user_balance.q4w.first_unchecked().amount >= 0);
    cvlr_assume!(user_balance.q4w.last_unchecked().amount >= 0);
    let sum_unchecked = user_balance.q4w.first_unchecked().amount + user_balance.q4w.last_unchecked().amount;

    user_balance.withdraw_shares(e, to_withdraw);

    cvlr_assert!(sum_unchecked >= to_withdraw );
}

/// withdraw_shares revert if called before exp; 
#[rule]
pub fn withdraw_shares_not_expired(e: &Env, to_withdraw: i128) {

    let mut user_balance:UserBalance = cvlr::nondet();
    cvlr_assume!(user_balance.q4w.len() == 2);
    let exp = user_balance.q4w.first_unchecked().exp;

    user_balance.withdraw_shares(e, to_withdraw);


    cvlr_assert!( exp <= e.ledger().timestamp());
}

/// withdraw_shares decrease total q4w amount
#[rule]
pub fn withdraw_shares_decrease_q4w_amount( e: &Env, to_withdraw: i128){

    let mut user_balance:UserBalance = cvlr::nondet();
    cvlr_assume!(user_balance.q4w.len() == 2);
    cvlr_assume!(user_balance.q4w.first_unchecked().amount >= 0);
    cvlr_assume!(user_balance.q4w.last_unchecked().amount >= 0);

    let total_q4w_before = user_balance.q4w.first_unchecked().amount + user_balance.q4w.last_unchecked().amount;
    
    clog!("before: ", total_q4w_before.clamp(i64::MIN as i128, i64::MAX as i128) as i64);

    user_balance.withdraw_shares(e, to_withdraw);

    let mut total_q4w_after = 0;
    
    if user_balance.q4w.len() == 2 {
        total_q4w_after = user_balance.q4w.first_unchecked().amount + user_balance.q4w.last_unchecked().amount;
    } else {
        total_q4w_after = user_balance.q4w.first_unchecked().amount
    }

    clog!("after: ", total_q4w_after.clamp(i64::MIN as i128, i64::MAX as i128) as i64);

    cvlr_assert!(total_q4w_before - to_withdraw == total_q4w_after);
}

/* -------------------------------------------------------------------------- */
/*                               dequeue_shares                               */
/* -------------------------------------------------------------------------- */

/// dequeue_shares decrease total q4w amount
#[rule]
pub fn dequeue_shares_decrease_q4w_amount( e: &Env, to_dequeue: i128){

    let mut user_balance:UserBalance = cvlr::nondet();
    cvlr_assume!(user_balance.q4w.len() == 2);
    cvlr_assume!(user_balance.q4w.first_unchecked().amount >= 0);
    cvlr_assume!(user_balance.q4w.last_unchecked().amount >= 0);

    let total_q4w_before = user_balance.q4w.first_unchecked().amount + user_balance.q4w.last_unchecked().amount;
    
    clog!("before: ", total_q4w_before.clamp(i64::MIN as i128, i64::MAX as i128) as i64);

    user_balance.dequeue_shares(e, to_dequeue);

    let mut total_q4w_after = 0;

    if user_balance.q4w.len() == 2 {
        total_q4w_after = user_balance.q4w.first_unchecked().amount + user_balance.q4w.last_unchecked().amount;
    } else {
        total_q4w_after = user_balance.q4w.first_unchecked().amount
    }

    clog!("after: ", total_q4w_after.clamp(i64::MIN as i128, i64::MAX as i128) as i64);

    cvlr_assert!(total_q4w_before - to_dequeue == total_q4w_after);
}

/// dequeue_shares pop if q4w amount is 0 
#[rule]
pub fn dequeue_shares_pop( e: &Env, to_dequeue: i128){

    let mut user_balance:UserBalance = cvlr::nondet();
    cvlr_assume!(user_balance.q4w.len() == 2);
    let last_q4w_amount_before = user_balance.q4w.last_unchecked().amount;
    cvlr_assume!(last_q4w_amount_before < to_dequeue);

    user_balance.dequeue_shares(e, to_dequeue);

    cvlr_assert!(user_balance.q4w.len() < 2);
}

/// dequeue_shares should less than total q4w amount
#[rule]
pub fn dequeue_shares_lte_total_q4w_amount( e: &Env, to_dequeue: i128){

    let mut user_balance:UserBalance = cvlr::nondet();
    cvlr_assume!(user_balance.q4w.len() == 2);

    cvlr_assume!(user_balance.q4w.first_unchecked().amount >= 0);
    cvlr_assume!(user_balance.q4w.last_unchecked().amount >= 0);
    let sum_unchecked = user_balance.q4w.first_unchecked().amount + user_balance.q4w.last_unchecked().amount;

    user_balance.dequeue_shares(e, to_dequeue);

    cvlr_assert!(sum_unchecked >= to_dequeue);
}