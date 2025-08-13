use cvlr::cvlr_assert_le;
use soroban_sdk::{Env, unwrap::UnwrapOptimized};
use cvlr::{cvlr_assert, cvlr_assume};
use cvlr_soroban_derive::rule;

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

// RULES FOR PRIVATE MUTANTS
//
// queue_shares_for_withdrawal RULES
// 1. CORRECTNESS RULES

// correcteness of queue_shares_for_withdrawal
#[rule]
pub fn queue_shares_for_withdrawal_correct(e: &Env, to_q: i128) {
    let mut user_balance: UserBalance = cvlr::nondet();
    cvlr_assume!(to_q <= user_balance.shares && user_balance.q4w.len() < MAX_Q4W_SIZE && to_q > 0);
    let user_shares_before = user_balance.shares;
    let q4w_before = user_balance.q4w.clone();
    
    user_balance.queue_shares_for_withdrawal(e, to_q);

    let user_shares_after = user_balance.shares;
    let q4w_after = user_balance.q4w.clone();
    let new_q4w = q4w_after.last().unwrap_optimized();

    cvlr_assert!(user_shares_after == user_shares_before - to_q);
    cvlr_assert!(q4w_after.len() == q4w_before.len() + 1);
    cvlr_assert!(new_q4w.amount == to_q);
    cvlr_assert!(new_q4w.exp == e.ledger().timestamp() + Q4W_LOCK_TIME);

}

// 2. ASSERTIONS CHECKS

// queue_shares_for_withdrawal should fail if queued amount > balance
#[rule]
pub fn queue_shares_balance_error_fails(e: &Env, to_q: i128) {
    let mut user_balance: UserBalance = cvlr::nondet();
    cvlr_assume!(to_q > user_balance.shares);
    user_balance.queue_shares_for_withdrawal(e, to_q);
    cvlr_assert!(false);
}

// queue_shares_for_withdrawal should fail if the maximum amount of active Q4W entries reached
#[rule]
pub fn queue_shares_too_many_q4w_fails(e: &Env, to_q: i128) {
    let mut user_balance: UserBalance = cvlr::nondet();
    cvlr_assume!(user_balance.q4w.len() >= MAX_Q4W_SIZE);
    user_balance.queue_shares_for_withdrawal(e, to_q);
    cvlr_assert!(false);
}

// withdraw_shares RULES
// 1. CORRECTNESS RULES

// withdraw_shares should decrease total amount of queued shares
#[rule]
pub fn withdraw_shares_decreases_q4w_total(e: &Env, to_withdraw: i128) {
    let mut user_balance: UserBalance = cvlr::nondet();
    let q4w_before = user_balance.q4w.clone();
    let available_shares_before: i128 = q4w_before.iter()
            .filter(|x| x.exp <= e.ledger().timestamp())
            .map(|x| x.amount)
            .sum();
    cvlr_assume!(to_withdraw > 0 && to_withdraw <= available_shares_before);

    user_balance.withdraw_shares(e, to_withdraw);
    
    let q4w_after = user_balance.q4w.clone();
    let available_shares_after: i128 = q4w_after.iter()
            .filter(|x| x.exp <= e.ledger().timestamp())
            .map(|x| x.amount)
            .sum();
    cvlr_assert!(available_shares_after == available_shares_before - to_withdraw);
}

// withdraw_shares should pop the earliest element in q4w 
// and pushes it in front if any amount left after execution
#[rule]
pub fn withdraw_shares_last_queued_after_first(e: &Env, to_withdraw: i128) {
    let mut user_balance: UserBalance = cvlr::nondet();
    let q4w_before = user_balance.q4w.clone();
    let first_in_cur_before = q4w_before.first_unchecked();
    let last_in_cur_before = q4w_before.last_unchecked();
    cvlr_assume!(first_in_cur_before.exp < last_in_cur_before.exp && to_withdraw < first_in_cur_before.amount);

    user_balance.withdraw_shares(e, to_withdraw);
    
    let q4w_after = user_balance.q4w.clone();

    let first_in_cur_after = q4w_after.first_unchecked().exp;
    let last_in_cur_after = q4w_after.last_unchecked().exp;
    cvlr_assert!(first_in_cur_after < last_in_cur_after);
}

// withdraw_shares shouldn't increace length of the q4w
#[rule]
pub fn withdraw_shares_length(e: &Env, to_withdraw: i128) {
    let mut user_balance: UserBalance = cvlr::nondet();
    let q4w_before = user_balance.q4w.clone();
    let l_before = q4w_before.len();
    user_balance.withdraw_shares(e, to_withdraw);
    let q4w_after = user_balance.q4w.clone();
    let l_after = q4w_after.len();
    cvlr_assert_le!(l_after, l_before);
}

// 2. ASSERTIONS CHECKS

// withdraw_shares should fail if amount > available shares in queue
#[rule]
pub fn withdraw_shares_balance_error_fails(e: &Env, to_withdraw: i128) {
    let mut user_balance: UserBalance = cvlr::nondet();
    let available_shares_before: i128 = user_balance.q4w.iter()
            .filter(|x| x.exp <= e.ledger().timestamp())
            .map(|x| x.amount)
            .sum();
    cvlr_assume!(to_withdraw > available_shares_before && available_shares_before > 0);
    user_balance.withdraw_shares(e, to_withdraw);
    cvlr_assert!(false);
}

// withdraw_shares should fail if there is no expired withdrawals
#[rule]
pub fn withdraw_shares_not_expired_fails(e: &Env, to_withdraw: i128) {
    let mut user_balance: UserBalance = cvlr::nondet();
    let q4w_before = user_balance.q4w.clone();
    let q4w_first = q4w_before.first().unwrap_optimized();
    cvlr_assume!(q4w_first.exp > e.ledger().timestamp());
    user_balance.withdraw_shares(e, to_withdraw);
    cvlr_assert!(false);
}

// dequeue_shares RULES
// 1. CORRECTNESS RULES

// dequeue_shares correctness
#[rule]
pub fn dequeue_shares_decreases_q4w_total(e: &Env, to_dequeue: i128) {
    let mut user_balance: UserBalance = cvlr::nondet();
    let q4w_before = user_balance.q4w.clone();
    let queued_shares_before: i128 = q4w_before.iter()
            .map(|x| x.amount)
            .sum();
    cvlr_assume!(to_dequeue > 0 && to_dequeue <= queued_shares_before);

    user_balance.dequeue_shares(e, to_dequeue);
    
    let q4w_after = user_balance.q4w.clone();
    let queued_shares_after: i128 = q4w_after.iter()
            .map(|x| x.amount)
            .sum();
    cvlr_assert!(queued_shares_after == queued_shares_before - to_dequeue);
}

// dequeue_shares shouldn't increace length of the q4w
#[rule]
pub fn dequeue_shares_length(e: &Env, to_dequeue: i128) {
    let mut user_balance: UserBalance = cvlr::nondet();
    let q4w_before = user_balance.q4w.clone();
    let l_before = q4w_before.len();
    user_balance.dequeue_shares(e, to_dequeue);
    let q4w_after = user_balance.q4w.clone();
    let l_after = q4w_after.len();
    cvlr_assert_le!(l_after, l_before);
}

// dequeue_shares should pop the latest element in q4w 
// and pushes it in bakc if any amount left after execution
#[rule]
pub fn dequeue_shares_last_queued_after_first(e: &Env, to_withdraw: i128) {
    let mut user_balance: UserBalance = cvlr::nondet();
    let q4w_before = user_balance.q4w.clone();
    let first_in_cur_before = q4w_before.first_unchecked();
    let last_in_cur_before = q4w_before.last_unchecked();
    cvlr_assume!(first_in_cur_before.exp < last_in_cur_before.exp && to_withdraw < last_in_cur_before.amount);

    user_balance.dequeue_shares(e, to_withdraw);
    
    let q4w_after = user_balance.q4w.clone();

    let first_in_cur_after = q4w_after.first_unchecked().exp;
    let last_in_cur_after = q4w_after.last_unchecked().exp;
    cvlr_assert!(first_in_cur_after < last_in_cur_after);
}

// 2. ASSERTIONS CHECKS

// Case: empty q4w -> queue x shares -> dequeue more than x shares
#[rule]
pub fn dequeue_more_than_queue_fails(e: &Env, to_q: i128, to_dequeue: i128) {
    let mut user_balance: UserBalance = cvlr::nondet();
    cvlr_assume!(user_balance.q4w.len() == 0 && to_q > 0 && to_q <= user_balance.shares && to_dequeue > to_q);
    user_balance.queue_shares_for_withdrawal(e, to_q);
    user_balance.dequeue_shares(e, to_dequeue);
    cvlr_assert!(false); //should pass
}

// dequeue_shares should fail if amount > available shares in queue
#[rule]
pub fn dequeue_shares_balance_error_fails(e: &Env, to_dequeue: i128) {
    let mut user_balance: UserBalance = cvlr::nondet();
    let queued_shares_before: i128 = user_balance.q4w.iter()
        .map(|x| x.amount)
        .sum();
    cvlr_assume!(to_dequeue > queued_shares_before && queued_shares_before > 0);
    user_balance.dequeue_shares(e, to_dequeue);
    cvlr_assert!(false);
}
