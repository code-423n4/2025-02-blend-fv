use cvlr::{clog, cvlr_assert, cvlr_assume};
use cvlr_soroban_derive::rule;
use cvlr_soroban::nondet_address;
use soroban_sdk::{Address, Env};
use sep_41_token::TokenClient;
use crate::storage;

//functions to call
use crate::backstop::{execute_queue_withdrawal, execute_dequeue_withdrawal, execute_withdraw};
use crate::certora_specs::{GHOST_UPDATE_EMISSIONS_CALLED, GHOST_FROM_BALANCE, GHOST_TO_BALANCE, GHOST_TO_RETURN}; 

    
// execute_queue_withdrawal() increases user sum of q4w by amount 
#[rule]
pub fn queue_increase_user_sum_q4w(e: &Env) { 

    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let shares: i128 = cvlr::nondet();
    //values before
    let user_q4w_before = storage::get_user_balance(e, &pool_address, &from).q4w;
    let user_q4w_lenght_before = user_q4w_before.len();
    cvlr_assume!(user_q4w_lenght_before == 2);
    clog!(user_q4w_lenght_before);
    let user_q4w_sum_before = user_q4w_before.get(0).unwrap().amount + user_q4w_before.get(1).unwrap().amount;

    execute_queue_withdrawal(e, &from, &pool_address, shares);

    //values after
    let user_sum_q4w_after = storage::get_user_balance(e, &pool_address, &from).q4w;
    let user_q4w_length_after = user_sum_q4w_after.len();
    clog!(user_q4w_length_after);

    let user_q4w_sum_after = user_sum_q4w_after.get(0).unwrap().amount + user_sum_q4w_after.get(1).unwrap().amount + user_sum_q4w_after.get(2).unwrap().amount;
    
    cvlr_assert!(user_q4w_sum_after == user_q4w_sum_before + shares);
}

// execute_withdraw() reverts if to_return is 0
#[rule]
pub fn withdraw_to_return_zero(e: &Env) {
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let shares: i128 = cvlr::nondet();
    cvlr_assume!(shares < 10); // to prevent timeouts
    let to_return: i128 = cvlr::nondet();
    cvlr_assume!(to_return == 0);
    unsafe{GHOST_TO_RETURN = to_return};
    
    execute_withdraw(e, &from, &pool_address, shares);
    cvlr_assert!(false);
}

// execute_withdraw() reduces pool.shares by amount, reduces pool.q4w by amount, reduces pool.tokens by tokensToWithdraw
#[rule]
pub fn withdraw_reduce_pool_tokens(e: &Env) {
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let shares: i128 = cvlr::nondet();
    cvlr_assume!(shares < 100); // to prevent timeouts
    let pool_before = storage::get_pool_balance(e, &pool_address);
    let to_return: i128 = cvlr::nondet();
    unsafe{GHOST_TO_RETURN = to_return};
    
    execute_withdraw(e, &from, &pool_address, shares);
    
    let pool_after = storage::get_pool_balance(e, &pool_address);
    
    cvlr_assert!(pool_after.tokens == pool_before.tokens - to_return);
}

// execute_withdraw() returns right amount of tokens to withdraw
#[rule]
pub fn withdraw_returns_tokens(e: &Env) {
        let from: Address = nondet_address();
        let pool_address: Address = nondet_address();
        let shares: i128 = cvlr::nondet();
        cvlr_assume!(shares < 100); // to prevent timeouts
        let to_return: i128 = cvlr::nondet();
        unsafe{GHOST_TO_RETURN = to_return};
        
        let result = execute_withdraw(e, &from, &pool_address, shares);
        
        cvlr_assert!(result == to_return);
    }
    
// execute_withdraw() reverts if shares > pool.shares || shares > pool.q4w
#[rule]
pub fn withdraw_amount_bigger_than_pool_shares(e: &Env) {
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let shares: i128 = cvlr::nondet();
    cvlr_assume!(shares < 100); // to prevent timeouts
    let pool = storage::get_pool_balance(e, &pool_address);
    cvlr_assume!(shares > pool.shares || shares > pool.q4w); 
    
    execute_withdraw(e, &from, &pool_address, shares);
    cvlr_assert!(false);
}

// execute_withdraw() reduces sum of user.q4w by amount, other users are untouched
#[rule]
pub fn withdraw_reduce_q4w_sum(e: &Env) {
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let shares: i128 = cvlr::nondet();

    //values before
    let user_q4w_before = storage::get_user_balance(e, &pool_address, &from).q4w;
    cvlr_assume!(user_q4w_before.len() == 2);
    let user_q4w_sum_before = user_q4w_before.iter().fold(0, |acc, x| acc + x.amount);
    
    execute_withdraw(e, &from, &pool_address, shares);

    //values after
    let user_sum_q4w_after = storage::get_user_balance(e, &pool_address, &from).q4w;
    let user_q4w_sum_after = user_sum_q4w_after.iter().fold(0, |acc, x| acc + x.amount);
    
    cvlr_assert!(user_q4w_sum_after == user_q4w_sum_before - shares);
}
    
// execute_dequeue_withdrawal() reverts if user does not have quewed withdraw request
#[rule]
pub fn dequeue_no_request(e: &Env) {
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let shares: i128 = cvlr::nondet();
    cvlr_assume!(shares > 0 && shares < 100); // to prevent timeouts
    let user_q4w = storage::get_user_balance(e, &pool_address, &from).q4w;
    cvlr_assume!(user_q4w.len() == 0);
    clog!(user_q4w.len() as i64);
    
    execute_dequeue_withdrawal(e, &from, &pool_address, shares);
    cvlr_assert!(false);
}

// execute_queue_withdrawal() reduces shares of user by amount, other users are untouched
#[rule]
pub fn queue_reduce_shares(e: &Env, from: Address, pool_address: Address, shares: i128, other_user: Address) { 
    //setup
    cvlr_assume!(from != other_user);
    // cvlr_assume!(shares < 3^32 -1 ); //make it usable for logs
    cvlr_assume!(shares == 15 ); //make it usable for logs

    //values before
    let user_shares_before = storage::get_user_balance(e, &pool_address, &from).shares;
    let other_user_shares_before = storage::get_user_balance(e, &pool_address, &other_user).shares;
    // cvlr_assume!(user_shares_before < 3^32 -1 ); //make it usable for logs
    cvlr_assume!(user_shares_before >= shares ); //make it usable for logs


    //LOGS
    clog!(shares as i64);
    clog!("user shares before", user_shares_before as i64);

                
    execute_queue_withdrawal(e, &from, &pool_address, shares);

    //values after
    let user_shares_after = storage::get_user_balance(e, &pool_address, &from).shares;
    let other_user_shares_after = storage::get_user_balance(e, &pool_address, &other_user).shares;
    clog!("user shares after", user_shares_after as i64);

    //assert
    cvlr_assert!(user_shares_after == user_shares_before - shares);
    cvlr_assert!(other_user_shares_after == other_user_shares_before);           
}

// execute_queue_withdrawal() increases length of userQuew by one
#[rule]
pub fn queue_increase_length(e: &Env, from: Address, pool_address: Address) {
        let shares: i128 = cvlr::nondet();
        let user_q4w_length_before = storage::get_user_balance(e, &pool_address, &from).q4w.len();
        clog!(shares as i64);
        clog!(user_q4w_length_before);
        
        execute_queue_withdrawal(e, &from, &pool_address, shares);

        let user_q4w_length_after = storage::get_user_balance(e, &pool_address, &from).q4w.len();
        clog!(user_q4w_length_after);
        
        cvlr_assert!(user_q4w_length_after == user_q4w_length_before + 1);
    }

// execute_withdraw() reduces balance of currentContract by tokensToWithdraw, increases tokensBalance of user by amount, other balances are untouched
#[rule]
pub fn withdraw_reduce_balance(e: &Env) {
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let shares: i128 = cvlr::nondet();

    //setup
    let pool_balance_before = cvlr::nondet();
    unsafe{GHOST_FROM_BALANCE = pool_balance_before};
    let pool_balance_before_ghost = unsafe{GHOST_FROM_BALANCE};

    let user_balance_before = cvlr::nondet();
    unsafe{GHOST_TO_BALANCE = user_balance_before};
    let user_balance_before_ghost = unsafe{GHOST_TO_BALANCE};
    
    let tokens_to_withdraw = execute_withdraw(e, &from, &pool_address, shares);

    //values after
    let pool_balance_after_ghost = unsafe{GHOST_FROM_BALANCE}; 
    let user_balance_after_ghost = unsafe{GHOST_TO_BALANCE};


    //assert
    cvlr_assert!(pool_balance_after_ghost == pool_balance_before_ghost - tokens_to_withdraw);
    cvlr_assert!(user_balance_after_ghost == user_balance_before_ghost + tokens_to_withdraw);
}

// execute_withdraw() reverts if amount is negative
#[rule]
pub fn withdraw_amount_negative(e: &Env, from: Address, pool_address: Address, shares: i128) {
    cvlr_assume!(shares < 0);
    execute_withdraw(e, &from, &pool_address, shares);
    cvlr_assert!(false);
}

// execute_withdraw() reverts if amount is bigger than sum of user q4w
#[rule]
pub fn withdraw_amount_bigger_than_sum(e: &Env, from: Address, pool_address: Address, shares: i128) {
    let user_q4w = storage::get_user_balance(e, &pool_address, &from).q4w;
    let sum = user_q4w.iter().fold(0, |acc, x| acc + x.amount);
    cvlr_assume!(shares > sum);
    
    execute_withdraw(e, &from, &pool_address, shares);
    cvlr_assert!(false);
}

// execute_withdraw() reverts if amount is more than unlocked q4w
#[rule]
pub fn withdraw_amount_bigger_than_unlocked(e: &Env, from: Address, pool_address: Address, shares: i128) {
    let user_q4w = storage::get_user_balance(e, &pool_address, &from).q4w;
    cvlr_assume!(user_q4w.len() == 3);
    //exp time is increasing
    cvlr_assume!(user_q4w.get(0).unwrap().exp <= user_q4w.get(1).unwrap().exp);
    cvlr_assume!(user_q4w.get(1).unwrap().exp <= user_q4w.get(2).unwrap().exp);
    let unlocked = user_q4w.iter().fold(0, |acc, x| if x.exp <= e.ledger().timestamp() {acc + x.amount} else {acc});
    cvlr_assume!(shares > unlocked);
    
    execute_withdraw(e, &from, &pool_address, shares);
    cvlr_assert!(false);
}


// execute_dequeue_withdrawal() reverts if amount is bigger than sum of quewed shares
#[rule]
pub fn dequeue_amount_bigger_than_sum(e: &Env, from: Address, pool_address: Address, shares: i128) {
    let user_q4w = storage::get_user_balance(e, &pool_address, &from).q4w;
    cvlr_assume!(user_q4w.len() < 3);
    
    clog!("LOG: user q4w 1:", user_q4w.get(0).unwrap().amount as i64);
    clog!("LOG: user q4w 1:", user_q4w.get(1).unwrap().amount as i64);
    clog!("LOG: user q4w 2:", user_q4w.get(2).unwrap().amount as i64);

    let sum = user_q4w.iter().fold(0, |acc, x| acc + x.amount);
    cvlr_assume!(shares == 32);
    cvlr_assume!(sum == 16);
    // cvlr_assume!(shares > sum);
    let divider = 2^64 as i128;
    let shares_first = shares / divider;
    clog!("LOG: shares first:", shares_first as i64);
    clog!("LOG: shares:", shares as i64);
    let sum_first = sum / divider;
    clog!("LOG: sum first:", sum_first as i64);
    clog!("LOG: sum:", sum as i64);
    
    execute_dequeue_withdrawal(e, &from, &pool_address, shares);
    clog!("LOG: Function did not revert");
    cvlr_assert!(false);
}

// execute_dequeue_withdrawal() reverts if requested shares are bigger than pool.q4w
#[rule]
pub fn dequeue_amount_bigger_than_pool(e: &Env, from: Address, pool_address: Address, shares: i128) {
    let pool_q4w = storage::get_pool_balance(e, &pool_address).q4w;
    cvlr_assume!(shares > pool_q4w);
    
    execute_dequeue_withdrawal(e, &from, &pool_address, shares);
    cvlr_assert!(false);
}

// execute_dequeue_withdrawal() removes as many withdraw request from the back as needed to cover amount, no other user is impacted
#[rule]
pub fn dequeue_remove_requests(e: &Env, from: Address, pool_address: Address, shares: i128, other_user: Address) {
    //setup
    cvlr_assume!(from != other_user);

    //values before
    let user_q4w_before = storage::get_user_balance(e, &pool_address, &from).q4w;
    cvlr_assume!(user_q4w_before.len() == 3);
    let user_sum_before = user_q4w_before.iter().fold(0, |acc, x| acc + x.amount);
    let mut shares_to_remove = shares;
    let mut removed_elements = 0;
    for index in user_q4w_before.len()-1..0 {
        //element is bigger than remaining shares
        if user_q4w_before.get(index).unwrap().amount > shares_to_remove {
            break;
        } else if user_q4w_before.get(index).unwrap().amount == shares_to_remove  {
            removed_elements += 1;
            break;
        } else {
            shares_to_remove -= user_q4w_before.get(index).unwrap().amount;
            removed_elements += 1;
        }
        
    }
    
    let other_user_q4w_before = storage::get_user_balance(e, &pool_address, &other_user).q4w;
    let other_user_length_before = other_user_q4w_before.len();
    let other_user_sum_before = other_user_q4w_before.iter().fold(0, |acc, x| acc + x.amount);
    
    execute_dequeue_withdrawal(e, &from, &pool_address, shares);

    //values after
    let user_q4w_after = storage::get_user_balance(e, &pool_address, &from).q4w;
    let user_q4w_length_after = user_q4w_after.len();
    let user_sum_after = user_q4w_after.iter().fold(0, |acc, x| acc + x.amount);
    let other_user_q4w_after = storage::get_user_balance(e, &pool_address, &other_user).q4w;
    let other_user_length_after = other_user_q4w_after.len();
    let other_user_sum_after = other_user_q4w_after.iter().fold(0, |acc, x| acc + x.amount);

    
    //assert
    cvlr_assert!(user_q4w_length_after == user_q4w_before.len() - removed_elements);
    cvlr_assert!(user_sum_after == user_sum_before - shares);
    cvlr_assert!(other_user_length_after == other_user_length_before);
    cvlr_assert!(other_user_sum_after == other_user_sum_before);
}

// execute_dequeue_withdrawal() does not increase the amount of the users q4w request
#[rule]
pub fn dequeue_does_not_increase_length(e: &Env, from: Address, pool_address: Address, shares: i128) {
    let user_q4w_length_before = storage::get_user_balance(e, &pool_address, &from).q4w.len();
    clog!("user q4w length before", user_q4w_length_before);
    
    execute_dequeue_withdrawal(e, &from, &pool_address, shares);

    let user_q4w_length_after = storage::get_user_balance(e, &pool_address, &from).q4w.len();
    clog!("user q4w length after", user_q4w_length_after);
    
    cvlr_assert!(user_q4w_length_after <= user_q4w_length_before);
}


// execute_dequeue_withdrawal() increases user.shares by amount, no other user is impacted
#[rule]
pub fn dequeue_increase_shares(e: &Env) {
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let other_user: Address = nondet_address();
    let shares:i128 = cvlr::nondet();

    //setup
    cvlr_assume!(from != other_user);

    //values before
    let user_shares_before = storage::get_user_balance(e, &pool_address, &from).shares;
    let other_user_shares_before = storage::get_user_balance(e, &pool_address, &other_user).shares;
    clog!(shares as i64);
    clog!("user shares before", user_shares_before as i64);
    
    execute_dequeue_withdrawal(e, &from, &pool_address, shares);

    //values after
    let user_shares_after = storage::get_user_balance(e, &pool_address, &from).shares;
    let other_user_shares_after = storage::get_user_balance(e, &pool_address, &other_user).shares;
    clog!("user shares after", user_shares_after as i64);

    //assert
    cvlr_assert!(user_shares_after == user_shares_before + shares);
    cvlr_assert!(other_user_shares_after == other_user_shares_before);           
}

// execute_dequeue_withdrawal() reduces the amount of shares q4w of user by amount
#[rule]
pub fn dequeue_reduce_user_sum_q4w(e: &Env, from: Address, pool_address: Address, shares: i128) {
    //values before
    let user_q4w_before = storage::get_user_balance(e, &pool_address, &from).q4w;
    let user_q4w_lenght_before = user_q4w_before.len();
    cvlr_assume!(user_q4w_lenght_before == 3);
    let user_q4w_sum_before = user_q4w_before.iter().fold(0, |acc, x| acc + x.amount);

    execute_dequeue_withdrawal(e, &from, &pool_address, shares);

    //values after
    let user_sum_q4w_after = storage::get_user_balance(e, &pool_address, &from).q4w;
    let user_q4w_sum_after = user_sum_q4w_after.iter().fold(0, |acc, x| acc + x.amount);

    cvlr_assert!(user_q4w_sum_after == user_q4w_sum_before - shares);
}

// execute_dequeue_withdrawal() reduces pool.q4w by amount^
#[rule]
pub fn dequeue_reduce_pool_q4w(e: &Env, from: Address, pool_address: Address, shares: i128) {
    let pool_q4w_before = storage::get_pool_balance(e, &pool_address).q4w;
    
    execute_dequeue_withdrawal(e, &from, &pool_address, shares);

    let pool_q4w_after = storage::get_pool_balance(e, &pool_address).q4w;
    
    cvlr_assert!(pool_q4w_after == pool_q4w_before - shares);
}

// execute_queue_withdrawal() reverts if user does not have enough shares 
#[rule]
pub fn queue_not_enough_shares(e: &Env) { 
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let shares : i128 = cvlr::nondet();
    // //create the values and set them in storage
    // let user_balance_start: UserBalance = cvlr::nondet();
    // storage::set_user_balance(e, &pool_address, &from, &user_balance_start); //@audit-issue testing at the moment
    
    let user_shares = storage::get_user_balance(e, &pool_address, &from).shares;
    let user_q4w = storage::get_user_balance(e, &pool_address, &from).q4w;
    //set up
    // cvlr_assume!(user_shares < shares);
    cvlr_assume!(user_shares == 16);
    cvlr_assume!(shares == 32);
    cvlr_assume!(user_q4w.len() == 2);

    //LOGS
    clog!("user q4w length before", user_q4w.len());
    clog!("LOG: shares:", shares as i64);
    clog!("LOG: user shares:", user_shares as i64);
    
    //function call
    execute_queue_withdrawal(e, &from, &pool_address, shares);

    let user_shares_after = storage::get_user_balance(e, &pool_address, &from).shares;
    clog!("LOG: user shares after:", user_shares_after as i64);
    let user_q4w_after = storage::get_user_balance(e, &pool_address, &from).q4w;
    let user_q4w_length_after = user_q4w_after.len();
    clog!("LOG: user q4w length after:", user_q4w_length_after as i64);

    //assert
    cvlr_assert!(false);
}

// execute_queue_withdrawal() returns the last/new withdrawal request
#[rule]
pub fn queue_returns_last_request(e: &Env) {
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let shares: i128 = cvlr::nondet(); 
    cvlr_assume!(shares == 16);
    let result = execute_queue_withdrawal(e, &from, &pool_address, shares);
    
    

    let user_q4w = storage::get_user_balance(e, &pool_address, &from).q4w;
    let length = user_q4w.len();
    clog!(length as i64);
    let last_request = user_q4w.get(length - 1).unwrap();

    //LOGS:
    clog!("LOG: result amount:", result.amount as i64);
    clog!("LOG: last request amount:", last_request.amount as i64);
    clog!(e.ledger().timestamp() as i64);
    clog!("LOG: result exp:", result.exp as i64);
    clog!("LOG: last request exp:", last_request.exp as i64);

    cvlr_assert!(result.amount == last_request.amount);
    cvlr_assert!(result.exp == last_request.exp);
}

// execute_queue_withdrawal() reverts if q4w is already >= MAX_Q4W_SIZE (20)
#[rule]
pub fn queue_max_q4w_size(e: &Env ) {
        let from: Address = nondet_address();
        let pool_address: Address = nondet_address();
        let shares: i128 = cvlr::nondet();
        let user_q4w_length = storage::get_user_balance(e, &pool_address, &from).q4w.len();
        cvlr_assume!(user_q4w_length >= 20);
        clog!(user_q4w_length);

        execute_queue_withdrawal(e, &from, &pool_address, shares);

        cvlr_assert!(false);
    }

// execute_queue_withdrawal() updates emmissions was called
#[rule]
pub fn queue_update_emissions_called(e: &Env){
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let shares: i128 = cvlr::nondet();
    
    execute_queue_withdrawal(e, &from, &pool_address, shares);

    cvlr_assert!(unsafe{GHOST_UPDATE_EMISSIONS_CALLED == true});
} 
    
// execute_dequeue_withdrawal() updates emissions was called
#[rule]
pub fn dequeue_update_emissions_called(e: &Env){
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let shares: i128 = cvlr::nondet();
    
    execute_dequeue_withdrawal(e, &from, &pool_address, shares);

    cvlr_assert!(unsafe{GHOST_UPDATE_EMISSIONS_CALLED == true});
}

// execute_dequeue_withdrawal() reverts if amount is negative
#[rule]
pub fn dequeue_amount_negative(e: &Env, from: Address, pool_address: Address, shares: i128) {
    cvlr_assume!(shares < 0);
    execute_dequeue_withdrawal(e, &from, &pool_address, shares);
    cvlr_assert!(false);
}
    

// execute_queue_withdrawal() unlock time of returned request is e.ledger().timestamp + Q4W_LOCK_TIME
#[rule]
pub fn queue_unlock_time_of_result(e: &Env, from: Address, pool_address: Address, shares: i128) {
    let unlock_time = e.ledger().timestamp() + 60 * 60 * 24 * 17; // Q4W_LOCK_TIME = 17 days
    
    let result = execute_queue_withdrawal(e, &from, &pool_address, shares);

    cvlr_assert!(result.exp == unlock_time);
}

// execute_queue_withdrawal() amount of returned request is amount
#[rule]
pub fn queue_amount_of_result(e: &Env, from: Address, pool_address: Address, shares: i128) {
    let result = execute_queue_withdrawal(e, &from, &pool_address, shares);

    cvlr_assert!(result.amount == shares);
}

// execute_queue_withdrawal() increases pool.q4w by amount 
#[rule]
pub fn queue_increase_pool_q4w(e: &Env, from: Address, pool_address: Address, shares: i128) {
    let pool_q4w_before = storage::get_pool_balance(e, &pool_address).q4w;
    
    execute_queue_withdrawal(e, &from, &pool_address, shares);

    let pool_q4w_after = storage::get_pool_balance(e, &pool_address).q4w;
    
    cvlr_assert!(pool_q4w_after == pool_q4w_before + shares);
}

// execute_queue_withdrawal() reverts if amount is nevative
#[rule]
pub fn queue_amount_negative(e: &Env, from: Address, pool_address: Address, shares: i128) {
    cvlr_assume!(shares < 0);
    execute_queue_withdrawal(e, &from, &pool_address, shares);
    cvlr_assert!(false);
}



//-------------------------------OLD RULES START----------------------------------

    // shares to withdraw must be nonnegative
    #[rule]
    pub fn amount_to_withdraw_nonnegative(e: &Env, from: Address, pool_address: Address, shares: i128) {
        cvlr_assume!(shares < 0);
        execute_withdraw(e, &from, &pool_address, shares);
        cvlr_assert!(false); // should pass when assumption is enabled, fail otherwise
    }

    // withdraw queue entries are all positive
    // needs -smt_preciseBitwiseOps
    #[rule]
    pub fn withdraw_queue_only_positive(e: Env, from: Address, pool_address: Address, amount: i128) {
        cvlr_assume!(amount < 0);
        execute_queue_withdrawal(&e, &from, &pool_address, amount);
        cvlr_assert!(false); // should pass when assumption is enabled, fail otherwise
    }

//-------------------------------OLD RULES END----------------------------------
