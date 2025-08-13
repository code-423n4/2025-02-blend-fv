use cvlr::{clog, cvlr_assert, cvlr_assume};
use cvlr_soroban_derive::rule;
use soroban_sdk::Env;

use crate::{backstop::UserBalance, constants::MAX_Q4W_SIZE, constants::Q4W_LOCK_TIME};


    //add_shares() increases shares by amount, nothing else changes
    #[rule]
    pub fn user_add_shares_increases_shares(e: &Env) {
        let mut user_balance: UserBalance = cvlr::nondet();
        let shares: i128 = cvlr::nondet();

        //values before
        let user_shares_before = user_balance.shares;
        let user_q4w_length_before = user_balance.q4w.len();

        //function call
        user_balance.add_shares(shares);

        //values after
        let user_shares_after = user_balance.shares;
        let user_q4w_length_after = user_balance.q4w.len();
        cvlr_assert!(user_shares_after == user_shares_before + shares);
        cvlr_assert!(user_q4w_length_after == user_q4w_length_before);
    }

    // dequeue_shares() if there is a rest, q4w is pushed to the back
    #[rule]
    pub fn user_dequeue_shares_rest_pushed_to_back(e: &Env) {
        // create call parameter
        let mut user_balance: UserBalance =cvlr::nondet();
        let to_dequeue: i128 = cvlr::nondet();
        cvlr_assume!(to_dequeue >= 0);
        let user_q4w_before = user_balance.q4w.clone();
        cvlr_assume!(user_q4w_before.len() == 3);
        
        //setup
        let amount0 = user_q4w_before.get(0).unwrap().amount;
        let amount1 = user_q4w_before.get(1).unwrap().amount;
        let amount2 = user_q4w_before.get(2).unwrap().amount;

        cvlr_assume!(amount0 == amount1 && amount1 == amount2); //same values for all q4w
        cvlr_assume!(amount0 > 2); // prevent error becasue of rounding down
        cvlr_assume!(to_dequeue == amount0 + amount0 / 2); // make sure 1,5 q4w are removed

        //function call
        user_balance.dequeue_shares(&e, to_dequeue);

        let user_balance_after = user_balance.q4w.clone();
        let amount0_after = user_balance_after.get(0).unwrap().amount;
        let amount1_after = user_balance_after.get(1).unwrap().amount;
        cvlr_assert!(amount0_after == amount0);
        cvlr_assert!(amount1_after == amount0 - (to_dequeue - amount0));
        }


    //withdraw_shares() if there is a rest, q4w is pushed back to the front
    #[rule]
    pub fn user_withdraw_shares_rest_pushed_to_front(e: &Env) {
        // create call parameter
        let mut user_balance: UserBalance =cvlr::nondet();
        let shares: i128 = cvlr::nondet();
        cvlr_assume!(shares >= 0);
        let user_q4w_before = user_balance.q4w.clone();
        cvlr_assume!(user_q4w_before.len() == 3);
        
        //setup
        let amount0 = user_q4w_before.get(0).unwrap().amount;
        let amount1 = user_q4w_before.get(1).unwrap().amount;
        let amount2 = user_q4w_before.get(2).unwrap().amount;

        cvlr_assume!(amount0 == amount1 && amount1 == amount2);
        cvlr_assume!(amount0 > 2); // prevent error becasue of rounding down
        cvlr_assume!(shares == amount0 + amount0 / 2); // make sure 1,5 q4w are removed
        
        //function call
        user_balance.withdraw_shares(&e, shares);

        let user_balance_after = user_balance.q4w.clone();

        let amount0_after = user_balance_after.get(0).unwrap().amount;
        let amount1_after = user_balance_after.get(1).unwrap().amount;
        cvlr_assert!(amount0_after == amount0 - (shares - amount0));
        cvlr_assert!(amount1_after == amount0);
    }
    
    // dequeue_shares() reverts if shares are more than sum of quewed shares
    #[rule]
    pub fn user_dequeue_shares_more_than_sum_of_queued_shares(e: &Env){
        // create call parameter
        let mut user_balance: UserBalance =cvlr::nondet();
        let to_dequeue: i128 = cvlr::nondet();
        cvlr_assume!(
            // to_dequeue > 0 && 
            to_dequeue < 100); // to_dequeue is positive
        clog!(to_dequeue as i64);
        let user_q4w_before = user_balance.q4w.clone();
        cvlr_assume!(user_q4w_before.len() == 3);
        
        // sum of queued shares
        let shares0: i128 = user_q4w_before.get(0).unwrap().amount;
        cvlr_assume!(shares0 > 0 && shares0 < 100); // "< 100" to prevent timeout
        let shares1 = user_q4w_before.get(1).unwrap().amount;
        cvlr_assume!(shares1 > 0 && shares1 < 100); // "< 100" to prevent timeout
        let shares2 = user_q4w_before.get(2).unwrap().amount;
        cvlr_assume!(shares2 > 0 && shares2 < 100); // "< 100" to prevent timeout
        let q4w_sum_share = shares0 + shares1 + shares2;
        cvlr_assume!(to_dequeue >  q4w_sum_share); // ensure revert of function call

        //LOG
        clog!(user_q4w_before.len() as i64);
        clog!(shares0 as i64);
        clog!(shares1 as i64);
        clog!(shares2 as i64);
        clog!(q4w_sum_share as i64);
        clog!(to_dequeue as i64);

        //function call
        user_balance.dequeue_shares(&e, to_dequeue);

        cvlr_assert!(false);           
    }

    // dequeue_shares() reduces shares from the back
    #[rule]
    pub fn user_dequewe_q4w_removed_from_back(e: &Env) {
        // create call parameter
        let mut user_balance: UserBalance =cvlr::nondet();
        let shares: i128 = cvlr::nondet();
        cvlr_assume!(shares >= 0);
        let user_q4w_before = user_balance.q4w.clone();
        cvlr_assume!(user_q4w_before.len() == 3);
        
        //setup
        let amount0 = user_q4w_before.get(0).unwrap().amount;
        let amount1 = user_q4w_before.get(1).unwrap().amount;
        let amount2 = user_q4w_before.get(2).unwrap().amount;

        cvlr_assume!(amount0 == shares && amount2 == shares);
        cvlr_assume!(amount1 != shares); // make value for nr1 different from other quewed shares
        
        //function call
        user_balance.dequeue_shares(&e, shares);

        let user_balance_after = user_balance.q4w.clone();
        let length = user_balance_after.len();
        cvlr_assert!(length == 2);

        let amount1_after = user_balance_after.get(1).unwrap().amount;
        cvlr_assert!(amount1_after == amount1);
    }

    // dequeue_shares() reduces sum of quewed share by shares
    #[rule]
    pub fn user_dequeue_shares_reduces_queued_shares(e: &Env){
        // create call parameter
        let mut user_balance: UserBalance =cvlr::nondet();
        let to_dequeue: i128 = cvlr::nondet();
        cvlr_assume!(to_dequeue >= 0); // to_dequeue is positive
        clog!(to_dequeue as i64);
        let user_q4w_before = user_balance.q4w.clone();
        cvlr_assume!(user_q4w_before.len() == 3);
        
        // sum of queued shares
        let shares0 = user_q4w_before.get(0).unwrap().amount;
        clog!(shares0 as i64);
        let shares1 = user_q4w_before.get(1).unwrap().amount;
        clog!(shares1 as i64);
        let shares2 = user_q4w_before.get(2).unwrap().amount;
        clog!(shares2 as i64);
        let q4w_sum_share = shares0 + shares1 + shares2;
        clog!(q4w_sum_share as i64);
        cvlr_assume!(shares0 < 100 && shares1 < 100 && shares2 < 100); // "> 100" to prevent timeout
        cvlr_assume!(to_dequeue <  q4w_sum_share); // ensure function call goes through

        //function call
        user_balance.dequeue_shares(&e, to_dequeue);

        let user_balance_after = user_balance.q4w.clone();
        let length = user_balance_after.len();
        let mut sum_after = 0;
        for i in 0..length{
            sum_after += user_balance_after.get(i).unwrap().amount;
            clog!(user_balance_after.get(i).unwrap().amount as i64);
        }
        clog!(sum_after as i64);
        
        cvlr_assert!(sum_after == q4w_sum_share - to_dequeue);           
    }

    // queue_shares_for_withdrawal() reduces user shares by shares
    #[rule]
    pub fn user_queue_shares_reduces_user_shares(e: &Env, shares: i128) {
        let mut user_balance: UserBalance = cvlr::nondet();
        cvlr_assume!(shares >= 0);
        let user_shares_before = user_balance.shares;
        cvlr_assume!(user_shares_before < 2^64);
        clog!(user_shares_before as u64);
        clog!(shares as u64);
        let length_before = user_balance.q4w.len();
        clog!(length_before);
        user_balance.queue_shares_for_withdrawal(&e, shares);
        let user_shares_after = user_balance.shares;
        clog!(user_shares_after as u64);
        let length_after = user_balance.q4w.len();
        clog!(length_after);
        cvlr_assert!(user_shares_after == user_shares_before - shares);
        
    }

    // withdraw_shares() revert if shares more than qued shares
    #[rule]
    pub fn user_withdraw_not_enough_shares_unlocked(e: &Env) {
        // create call parameter
        let mut user_balance: UserBalance =cvlr::nondet();
        let shares: i128 = cvlr::nondet();
        cvlr_assume!(shares >= 0);
        let current_timestamp = e.ledger().timestamp();
        let user_q4w_before = user_balance.q4w.clone();
        cvlr_assume!(user_q4w_before.len() == 3);
        
        // exp are increasing
        let exp_0 = user_q4w_before.get(0).unwrap().exp;
        let exp_1 = user_q4w_before.get(1).unwrap().exp;
        let exp_2 = user_q4w_before.get(2).unwrap().exp;
        cvlr_assume!(exp_0 < exp_1);
        cvlr_assume!(exp_1 < exp_2);
        //ensure all quewed shares are unlocked
        cvlr_assume!(exp_0 <= current_timestamp);
        cvlr_assume!(exp_1 <= current_timestamp);
        cvlr_assume!(exp_2 > current_timestamp);

        let shares0 = user_q4w_before.get(0).unwrap().amount;
        cvlr_assume!(shares0 > 0 && shares0 < 100); // "> 100" to prevent timeout
        let shares1 = user_q4w_before.get(1).unwrap().amount;
        cvlr_assume!(shares1 > 0 && shares1 < 100); // "> 100" to prevent timeout
        let shares2 = user_q4w_before.get(2).unwrap().amount;
        cvlr_assume!(shares2 > 0 && shares2 < 100); // "> 100" to prevent timeout

        let q4w_sum_share = shares0 + shares1 + shares2;

            //LOG
        clog!(user_q4w_before.len() as i64);
        clog!(e.ledger().timestamp());
        clog!(exp_0);
        clog!(shares0 as i64);
        clog!(exp_1);
        clog!(shares1 as i64);
        clog!(exp_2);
        clog!(shares2 as i64);
        clog!(shares as i64);
        clog!(q4w_sum_share as i64);
        clog!(shares0 as i64);
        clog!(shares1 as i64);
        clog!(shares2 as i64);

        //setup for revert
        cvlr_assume!(q4w_sum_share == shares);

        //function call
        user_balance.withdraw_shares(&e, shares);

        cvlr_assert!(false);
    }

    // withdraw_shares() length of q4w does not increase
    #[rule]
    pub fn user_withdraw_length_of_q4w_does_not_increase(e: &Env) {
        // create call parameter
        let mut user_balance: UserBalance =cvlr::nondet();
        let shares: i128 = cvlr::nondet();
        cvlr_assume!(shares >= 0);
        let user_q4w_before = user_balance.q4w.clone();
        cvlr_assume!(user_q4w_before.len() == 3);
        
        //function call
        user_balance.withdraw_shares(&e, shares);

        let user_balance_after = user_balance.q4w.clone();
        let length = user_balance_after.len();
        cvlr_assert!(length <= 3);
    }
    
    
    // withdraw_shares() q4w is removed from the front
    #[rule]
    pub fn user_withdraw_q4w_removed_from_front(e: &Env) {
        // create call parameter
        let mut user_balance: UserBalance =cvlr::nondet();
        let shares: i128 = cvlr::nondet();
        cvlr_assume!(shares >= 0);
        let user_q4w_before = user_balance.q4w.clone();
        cvlr_assume!(user_q4w_before.len() == 3);
        
        //setup
        let amount0 = user_q4w_before.get(0).unwrap().amount;
        let amount1 = user_q4w_before.get(1).unwrap().amount;
        let amount2 = user_q4w_before.get(2).unwrap().amount;

        cvlr_assume!(amount0 == shares && amount2 == shares);
        cvlr_assume!(amount1 != shares); // make value for nr1 different from other quewed shares
        
        //function call
        user_balance.withdraw_shares(&e, shares);

        let user_balance_after = user_balance.q4w.clone();
        let length = user_balance_after.len();
        cvlr_assert!(length == 2);

        let amount0_after = user_balance_after.get(0).unwrap().amount;
        cvlr_assert!(amount0_after == amount1);
    }
    
    // withdraw_shares() sum q4w is reduced by shares
    #[rule]
    pub fn user_withdraw_sum_q4w_reduced_by_shares(e: &Env) {
        // create call parameter
        let mut user_balance: UserBalance =cvlr::nondet();
        let shares: i128 = cvlr::nondet();
        cvlr_assume!(shares >= 0);
        let user_q4w_before = user_balance.q4w.clone();
        cvlr_assume!(user_q4w_before.len() == 3);
        
        // exp are increasing
        let exp_0 = user_q4w_before.get(0).unwrap().exp;
        let exp_1 = user_q4w_before.get(1).unwrap().exp;
        let exp_2 = user_q4w_before.get(2).unwrap().exp;
        cvlr_assume!(exp_0 < exp_1);
        cvlr_assume!(exp_1 < exp_2);

        let shares0 = user_q4w_before.get(0).unwrap().amount;
        let shares1 = user_q4w_before.get(1).unwrap().amount;
        let shares2 = user_q4w_before.get(2).unwrap().amount;
        cvlr_assume!(shares0 > 0 && shares1 > 0 && shares2 > 0);
        cvlr_assume!(shares0 < 100 && shares1 < 100 && shares2 < 100); // "> 100" to prevent timeout


        let q4w_sum_share = shares0 + shares1 + shares2;

        //function call
        user_balance.withdraw_shares(&e, shares);

        let user_balance_after = user_balance.q4w.clone();
        let length = user_balance_after.len();
        let mut sum_after = 0;
        for i in 0..length{
            clog!(user_balance_after.get(i).unwrap().amount as i64);
            sum_after += user_balance_after.get(i).unwrap().amount;
        }
        
        //assert
        cvlr_assert!(sum_after == q4w_sum_share - shares);
    }

    // withdraw_shares() reverts if shares more than sum of unlocked shares
    #[rule]
    pub fn user_withdraw_shares_more_than_sum_of_unlocked_shares(e: &Env) {
        //create call parameter
        let mut user_balance: UserBalance =cvlr::nondet();
        let shares: i128 = cvlr::nondet();

        cvlr_assume!(shares >= 0);
        let user_q4w_before = user_balance.q4w.clone();
        let length = user_q4w_before.len();
        cvlr_assume!(length == 3); // 3 withdrawals are queued
        
        // exp are increasing so loop goes through //@audit should not be necessary
        let exp_0 = user_q4w_before.get(0).unwrap().exp;
        let exp_1 = user_q4w_before.get(1).unwrap().exp;
        let exp_2 = user_q4w_before.get(2).unwrap().exp;
        cvlr_assume!(exp_0 < exp_1);
        cvlr_assume!(exp_1 < exp_2);

        //shares to q4w (less than 100 to prevent timeout)
        let shares0 = user_q4w_before.get(0).unwrap().amount;
        cvlr_assume!(shares0 < 100 && shares0 > 0);
        let shares1 = user_q4w_before.get(1).unwrap().amount;
        cvlr_assume!(shares1 < 100 && shares1 > 0);
        let shares2 = user_q4w_before.get(2).unwrap().amount;
        cvlr_assume!(shares2 < 100 && shares2 > 0);

        let unlocked_shares0 = if exp_0 <= e.ledger().timestamp()  {user_q4w_before.get(0).unwrap().amount} else {0};
        let unlocked_shares1 = if exp_1 <= e.ledger().timestamp()  {user_q4w_before.get(1).unwrap().amount} else {0};
        let unlocked_shares2 = if exp_2 <= e.ledger().timestamp()  {user_q4w_before.get(2).unwrap().amount} else {0}; 
        
        let q4w_sum_unlocked = unlocked_shares0 + unlocked_shares1 + unlocked_shares2;

        //setup for revert
        cvlr_assume!(q4w_sum_unlocked < shares);

        // //LOG
        //     clog!(length as i64);
        //     clog!(e.ledger().timestamp());
        //     clog!(exp_0);
        //     clog!(shares0 as i64);
        //     clog!(exp_1);
        //     clog!(shares1 as i64);
        //     clog!(exp_2);
        //     clog!(shares2 as i64);
        //     clog!(shares as i64);
        //     clog!(q4w_sum_unlocked as i64);
        //     clog!(unlocked_shares0 as i64);
        //     clog!(unlocked_shares1 as i64);
        //     clog!(unlocked_shares2 as i64);

        //function call
        user_balance.withdraw_shares(&e, shares);

        cvlr_assert!(false);
    }


    // queue_shares_for_withdrawal() reverts if user shares are less than shares
    #[rule]
    pub fn user_queue_user_shares_less_than_shares(e: &Env, user_balance: &mut UserBalance, shares: i128) {
        cvlr_assume!(shares >= 0);
        let user_shares_before = user_balance.shares;
        cvlr_assume!(user_shares_before < shares);
        user_balance.queue_shares_for_withdrawal(&e, shares);
        cvlr_assert!(false);
    }

    // queue_shares_for_withdrawal() reverts if q4w.len() >= MAX_Q4W_SIZE
    #[rule]
    pub fn user_queue_max_q4w_size(e: &Env, user_balance: &mut UserBalance, shares: i128) {
        cvlr_assume!(user_balance.q4w.len() >= MAX_Q4W_SIZE);
        user_balance.queue_shares_for_withdrawal(&e, shares);
        cvlr_assert!(false);
    }

    // queue_shares_for_withdrawal() increases q4w.length by 1
    #[rule]
    pub fn user_queue_increases_q4w_length(e: &Env, user_balance: &mut UserBalance, shares: i128) {
        cvlr_assume!(shares >= 0);
        let q4w_len_before = user_balance.q4w.len();
        user_balance.queue_shares_for_withdrawal(&e, shares);
        let q4w_len_after = user_balance.q4w.len();
        cvlr_assert!(q4w_len_after == q4w_len_before + 1);
    }

    // queue_shares_for_withdrawal() sets amount of new q4w to shares
    #[rule]
    pub fn user_queue_sets_amount_to_shares(e: &Env, user_balance: &mut UserBalance, shares: i128) {
        cvlr_assume!(shares >= 0);
        
        user_balance.queue_shares_for_withdrawal(&e, shares);
        
        let return_q4w = user_balance.q4w.last().unwrap();
        
        cvlr_assert!(return_q4w.amount == shares);
    }

    // queue_shares_for_withdrawal() sets exp of new q4w to timestamp() + Q4W_LOCK_TIME
    #[rule]
    pub fn user_queue_sets_exp_to_timestamp_plus_q4w_lock_time(e: &Env, user_balance: &mut UserBalance, shares: i128) {
    cvlr_assume!(shares >= 0);
    
    user_balance.queue_shares_for_withdrawal(&e, shares);
    
    let return_q4w = user_balance.q4w.last().unwrap();
    
    cvlr_assert!(return_q4w.exp == e.ledger().timestamp() + Q4W_LOCK_TIME);
}




//------------------------- OLD RULES START -------------------------

    // deposit should increase user shares
    #[rule]
    pub fn add_shares_increases_user_shares(user_balance: &mut UserBalance, shares: i128) {
        cvlr_assume!(shares >= 0);
        let user_shares_before = user_balance.shares;
        user_balance.add_shares(shares);
        let user_shares_after = user_balance.shares;
        cvlr_assert!(user_shares_after == user_shares_before + shares);
    }

//------------------------- OLD RULES END -------------------------
