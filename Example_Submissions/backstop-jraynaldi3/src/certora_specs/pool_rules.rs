use crate::backstop::require_is_from_pool_factory;
use crate::certora_specs::mocks::{
    conversions::{certora_convert_to_shares, certora_convert_to_tokens},
    pool_factory::PoolFactoryClient
};
use crate::storage;
use crate::PoolBalance;
use blend_contract_sdk::pool;
use cvlr::{clog, cvlr_satisfy};
use cvlr::nondet::cvlr_nondet_small_i128;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};

use cvlr::asserts::{cvlr_assert, cvlr_assume};

use super::mocks::pool_factory::init_is_pool_ghost;

// converting 0 units of tokens/shares will lead to 0 units
#[rule]
pub fn conversion_of_zero(pool_balance: &mut PoolBalance) {
    let tokens = pool_balance.convert_to_tokens(0);
    let shares = pool_balance.convert_to_shares(0);
    cvlr_assert!(tokens == 0 && shares == 0);
}

// token -> shares gives tokens when pool has 0 shares
#[rule]
pub fn conversion_pool_zero_shares(pool_balance: &mut PoolBalance, tokens: i128) {
    cvlr_assume!(pool_balance.shares == 0); //, "pool has 0 shares");
    let shares = pool_balance.convert_to_shares(tokens);
    cvlr_assert!(shares == tokens);
    cvlr_assert!(pool_balance.shares == 0);
}

// simpler correctness of token conversion for i64 variant
#[rule]
pub fn simple_token_roundtrip_correct(pool_shares: i64, pool_tokens: i64, tokens: i64) {
    cvlr_assume!(
        tokens >= 0 && pool_shares > 0 && pool_tokens > 0);
    let tokens_res = certora_convert_to_tokens(
        pool_shares,
        pool_tokens,
        certora_convert_to_shares(pool_shares, pool_tokens, tokens),
    );
    cvlr_assert!(tokens >= tokens_res);
}

// simpler correctness of share conversion for i64 variant
#[rule]
pub fn simple_share_roundtrip_correct(pool_shares: i64, pool_tokens: i64, shares: i64) {
    cvlr_assume!(
        shares >= 0 && pool_shares > 0 && pool_tokens > 0); 
    let shares_res = certora_convert_to_shares(
        pool_shares,
        pool_tokens,
        certora_convert_to_tokens(pool_shares, pool_tokens, shares),
    );
    cvlr_assert!(shares >= shares_res);
}

/* -------------------------------------------------------------------------- */
/*                                 conversion                                 */
/* -------------------------------------------------------------------------- */

/// conversion monotonicity shares to token to shares
#[rule]
pub fn conversion_monotonicity_sts(
    e: Env,
    num: i128
) {
    let pool_balance: PoolBalance = cvlr::nondet();
    cvlr_assume!(pool_balance.tokens > 0 && pool_balance.shares > 0 && num > 0);

    let num_res = pool_balance.convert_to_shares(
        pool_balance.convert_to_tokens(num)
    );

    cvlr_assert!(num >= num_res);
}

/// conversion monotonicity token to shares to token
#[rule]
pub fn conversion_monotonicity_tst(
    e: Env,
    num: i128
) {
    let pool_balance: PoolBalance = cvlr::nondet();
    cvlr_assume!(pool_balance.tokens > 0 && pool_balance.shares > 0 && num > 0);
    
    let num_res = pool_balance.convert_to_tokens(
        pool_balance.convert_to_shares(num)
    );

    cvlr_assert!(num >= num_res);
}

/* -------------------------------------------------------------------------- */
/*                                   deposit                                  */
/* -------------------------------------------------------------------------- */

/// deposit should add tokens 
#[rule]
pub fn deposit_add_tokens(tokens:i128){
    let mut pool_balance: PoolBalance = cvlr::nondet();
    let shares:i128 = cvlr_nondet_small_i128();
    let tokens_before = pool_balance.tokens;

    pool_balance.deposit(tokens, shares);

    let tokens_after = pool_balance.tokens;
    cvlr_assert!(tokens_after == tokens_before + tokens);
}

/// deposit should add shares 
#[rule]
pub fn deposit_add_shares(shares:i128){
    let mut pool_balance: PoolBalance = cvlr::nondet();
    let tokens:i128 = cvlr_nondet_small_i128();
    let shares_before = pool_balance.shares;

    pool_balance.deposit(tokens, shares);

    let shares_after = pool_balance.shares;
    cvlr_assert!(shares_after == shares_before + shares);
}

/* -------------------------------------------------------------------------- */
/*                                  withdraw                                  */
/* -------------------------------------------------------------------------- */

/// withdraw should reduce tokens
#[rule]
pub fn withdraw_reduce_tokens(e:&Env, tokens:i128){
    let mut pool_balance: PoolBalance = cvlr::nondet();
    let shares:i128 = cvlr_nondet_small_i128();
    let tokens_before = pool_balance.tokens;

    pool_balance.withdraw(e, tokens, shares);

    let tokens_after = pool_balance.tokens;
    cvlr_assert!(tokens_after == tokens_before - tokens);
}

/// withdraw should reduce shares
#[rule]
pub fn withdraw_reduce_shares(e:&Env, shares:i128){
    let mut pool_balance: PoolBalance = cvlr::nondet();
    let tokens:i128 = cvlr_nondet_small_i128();
    let shares_before = pool_balance.shares;

    pool_balance.withdraw(e, tokens, shares);

    let shares_after = pool_balance.shares;
    cvlr_assert!(shares_after == shares_before - shares);
}

/// withdraw should reduce q4w
#[rule]
pub fn withdraw_reduce_q4w(e:&Env, shares:i128){
    let mut pool_balance: PoolBalance = cvlr::nondet();
    let tokens:i128 = cvlr_nondet_small_i128();
    let q4w_before = pool_balance.q4w;

    pool_balance.withdraw(e, tokens, shares);

    let q4w_after = pool_balance.q4w;
    cvlr_assert!(q4w_after == q4w_before - shares);
}

/// tokens, shares, and q4w should always positive 
#[rule]
pub fn withdraw_positive_value(e:&Env, tokens:i128, shares:i128){
    let mut pool_balance: PoolBalance = cvlr::nondet();

    pool_balance.withdraw(e, tokens, shares);

    cvlr_assert!(
        pool_balance.q4w >= 0
        && pool_balance.shares >= 0
        && pool_balance.tokens >= 0
    );
}

/* -------------------------------------------------------------------------- */
/*                             queue_for_withdraw                             */
/* -------------------------------------------------------------------------- */
/// queue_for_withdraw should add q4w
#[rule]
pub fn queue_for_withdraw_add_q4w(e:&Env, shares:i128){
    let mut pool_balance: PoolBalance = cvlr::nondet();
    let q4w_before = pool_balance.q4w;

    pool_balance.queue_for_withdraw(shares);

    let q4w_after = pool_balance.q4w;
    cvlr_assert!(q4w_after == q4w_before + shares);
}

/* -------------------------------------------------------------------------- */
/*                                 dequeue_q4w                                */
/* -------------------------------------------------------------------------- */
/// dequeue_q4w should reduce q4w
#[rule]
pub fn dequeue_q4w_reduce_q4w(e:&Env, shares:i128){
    let mut pool_balance: PoolBalance = cvlr::nondet();
    let q4w_before = pool_balance.q4w;

    pool_balance.dequeue_q4w(e, shares);

    let q4w_after = pool_balance.q4w;
    cvlr_assert!(q4w_after == q4w_before - shares);
}

/// q4w should always positive
#[rule]
pub fn dequeue_q4w_positive_value(e:&Env, shares:i128){
    let mut pool_balance: PoolBalance = cvlr::nondet();

    pool_balance.dequeue_q4w(e, shares);

    cvlr_assert!(pool_balance.q4w >= 0);
}

/* -------------------------------------------------------------------------- */
/*                        require_is_from_pool_factory                        */
/* -------------------------------------------------------------------------- */

/// require is from pool factory only needed when balance is 0 
#[rule] 
pub fn require_is_from_pool_factory_balance_zero_only(e: &Env, address: &Address, balance: i128) {
    init_is_pool_ghost(address);
    let pool_factory_client = PoolFactoryClient::new(e, &storage::get_pool_factory(e));
    cvlr_assume!(balance > 0);
    cvlr_assume!(!pool_factory_client.is_pool(address));
    require_is_from_pool_factory(e,address, balance);
    cvlr_satisfy!(true);
}

/// require should check is pool when balance == 0 
#[rule] 
pub fn require_is_from_pool_factory_is_pool_check(e: &Env, address: &Address, balance: i128) {
    init_is_pool_ghost(address);
    let pool_factory_client = PoolFactoryClient::new(e, &storage::get_pool_factory(e));
    cvlr_assume!(balance == 0);
    cvlr_assume!(!pool_factory_client.is_pool(address));
    require_is_from_pool_factory(e,address, balance);
    cvlr_assert!(false);
}