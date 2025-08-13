use crate::backstop::load_pool_backstop_data;
use crate::backstop::require_is_from_pool_factory;
use crate::backstop::require_pool_above_threshold;
use crate::certora_specs::mocks::conversions::certora_convert_to_shares;
use crate::certora_specs::mocks::conversions::certora_convert_to_tokens;
use crate::certora_specs::summaries::rounding;
use crate::backstop::PoolBackstopData;
use crate::certora_specs::summaries::rounding::fixed_div_ceil;
use crate::certora_specs::GHOST_BLND;
use crate::certora_specs::GHOST_BLND_PER_TOKEN;
use crate::certora_specs::GHOST_IS_POOL; //@audit added for ghost

use crate::certora_specs::GHOST_USDC_PER_TOKEN;
use crate::certora_specs::{GHOST_POOL_TOTAL_SUPPLY, GHOST_POOL_BLND_BALANCE, GHOST_POOL_USDC_BALANCE }; //@audit added for ghost
use crate::constants::SCALAR_7;
use crate::dependencies::CometClient;
use crate::dependencies::PoolFactoryClient;
use crate::storage;
use crate::PoolBalance;
use cvlr::clog;
use cvlr::cvlr_satisfy;
use cvlr::nondet;
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};
// use crate::backstop::GHOST_IS_POOL;

use cvlr::asserts::{cvlr_assert, cvlr_assume};


//------------------REUSABLE FUNCTIONS START------------------

// pool_balance sanity
pub fn pool_balance_sanity(pool_balance: &mut PoolBalance) {
    cvlr_assume!(pool_balance.tokens >= 0);
    cvlr_assume!(pool_balance.shares >= 0);
    cvlr_assume!(pool_balance.q4w >= 0);
    cvlr_assume!(pool_balance.tokens < 2^32);
    cvlr_assume!(pool_balance.shares < 2^32);
    cvlr_assume!(pool_balance.q4w < 2^32);
    cvlr_assume!(pool_balance.q4w <= pool_balance.shares);
    cvlr_assume!(pool_balance.convert_to_tokens(pool_balance.q4w) <= pool_balance.tokens);
}

//------------------REUSABLE FUNCTIONS END------------------

   
    // load_pool_backstop_data() returns right blnd
    #[rule]
    pub fn load_pool_backstop_returns_blnd(e: &Env) {
        let address: Address = nondet_address();
        let pool_balance = storage::get_pool_balance(e, &address);

        //setup
        cvlr_assume!(pool_balance.shares == 0); //skip first if statement
        cvlr_assume!(pool_balance.tokens > 0 && pool_balance.tokens < 100); //enter second if statement
        

        //target
        let blnd_per_token: i128 = cvlr::nondet();
        unsafe{GHOST_BLND_PER_TOKEN = blnd_per_token;}
        cvlr_assume!(blnd_per_token > 0 && blnd_per_token < 100); //prevent time out
        
        //blnd correstponding to the LP shares held by the pool
        let target = rounding::fixed_mul_floor(pool_balance.tokens, blnd_per_token, SCALAR_7);
        clog!(blnd_per_token as i64);
        clog!(target as i64);

        
        //setup to prevent time out
        ////@audit set USDC per tokne to be 0 to reduce resources needed
        // let usdc_token = storage::get_usdc_token(e); 
        // let total_usdc = comet_client.get_balance(&usdc_token);
        // cvlr_assume!(total_usdc == 0); // //@audit prevent time out
        
        clog!(pool_balance.tokens as i64);

        let restult = load_pool_backstop_data(e, &address);

        clog!(pool_balance.tokens as i64);
        clog!(blnd_per_token as i64);
        clog!(restult.blnd as i64);

        cvlr_assert!(restult.blnd == target);
    }

    // load_pool_backstop_data() returns right usdc
    #[rule]
    pub fn load_pool_backstop_returns_usdc(e: &Env) {
        let address: Address = nondet_address();
        let pool_balance = storage::get_pool_balance(e, &address);

        //setup
        cvlr_assume!(pool_balance.shares == 0); //skip first if statement
        cvlr_assume!(pool_balance.tokens > 0 && pool_balance.tokens < 100); //enter second if statement
        

        //target
        let usdc_per_token: i128 = cvlr::nondet();
        // unsafe{GHOST_POOL_BLND_BALANCE = usdc_per_token;} //@audit-issue works
        unsafe{GHOST_USDC_PER_TOKEN = usdc_per_token;}
        cvlr_assume!(usdc_per_token > 0 && usdc_per_token < 100); //prevent time out
        
        //blnd correstponding to the LP shares held by the pool
        let target = rounding::fixed_mul_floor(pool_balance.tokens, usdc_per_token, SCALAR_7);
        clog!(target as i64);

        // let target = cvlr::nondet(); //@audit-issue works but does not prove that much
        // unsafe{GHOST_BLND = target;} //@audit-issue works but does not prove that much
        
        //setup to prevent time out
        ////@audit set USDC per tokne to be 0 to reduce resources needed
        // let usdc_token = storage::get_usdc_token(e); 
        // let total_usdc = comet_client.get_balance(&usdc_token);
        // cvlr_assume!(total_usdc == 0); // //@audit prevent time out
        
        clog!(pool_balance.tokens as i64);

        let restult = load_pool_backstop_data(e, &address);

        clog!(pool_balance.tokens as i64);
        clog!(usdc_per_token as i64);
        clog!(restult.usdc as i64);

        cvlr_assert!(restult.usdc == target);
    }
    
    // load_pool_backstop_data() retunrs q4w/shares ceiling if shares are not 0 (remiander)
    #[rule]
    pub fn load_pool_backstop_returns_q4w_pct(e: &Env) { 
        let address: Address = nondet_address();

        let pool_balance = storage::get_pool_balance(e, &address);

        //setup
        cvlr_assume!(pool_balance.shares > 0); // enter first if statement
        cvlr_assume!(pool_balance.shares < 2000); //prevent time out
        // cvlr_assume!(pool_balance.tokens == 0); //skip second if statement
        cvlr_assume!(pool_balance.q4w > 0 && pool_balance.q4w < 10000); //prevent time out

        let target_q4w_pct = rounding::fixed_div_ceil(pool_balance.q4w, pool_balance.shares, SCALAR_7);

        let restult = load_pool_backstop_data(e, &address);

        cvlr_assert!(restult.q4w_pct == target_q4w_pct);
    }
     
    // load_pool_backstop_data() returns pool_balance.tokens for tokens
    #[rule]
    pub fn load_pool_backstop_returns_tokens(e: &Env) {
        let address: Address = nondet_address();

        let pool_balance = storage::get_pool_balance(e, &address);

        //setup
        cvlr_assume!(pool_balance.tokens > 0);

        let restult = load_pool_backstop_data(e, &address);

        cvlr_assert!(restult.tokens == pool_balance.tokens);
    }

    // load_pool_backstop_data() returns all 0 if pool_tokens = 0 && pool_shares == 0
    #[rule]
    pub fn load_pool_backstop_returns_zero(e: &Env) {
        let address: Address = nondet_address();

        let pool_balance = storage::get_pool_balance(e, &address);

        //setup
        cvlr_assume!(pool_balance.shares == 0 && pool_balance.tokens == 0);
     
        let restult = load_pool_backstop_data(e, &address);

        cvlr_assert!(restult.tokens == 0);
        cvlr_assert!(restult.q4w_pct == 0);
        cvlr_assert!(restult.blnd == 0);
        cvlr_assert!(restult.usdc == 0);
    }

    // require_pool_above_threshold() returns false if the pool is below 100%
    #[rule]
    pub fn require_pool_above_threshold_returns_false(e: &Env) {
        let threshold_pc = 10_000_000_000_000_000_000_000_000i128;
        let pool_backstop_data:PoolBackstopData = nondet(); 
        let whole_blnd = pool_backstop_data.blnd/ SCALAR_7;
        let whole_usdc = pool_backstop_data.usdc/ SCALAR_7;
        let saturating_pool_pc = whole_blnd
        .saturating_mul(whole_blnd)
        .saturating_mul(whole_blnd)
        .saturating_mul(whole_blnd)
        .saturating_mul(whole_usdc);

        cvlr_assume!(saturating_pool_pc < threshold_pc);

        //function call
        let result = require_pool_above_threshold(&pool_backstop_data);

        cvlr_assert!(result == false);
    }

    // require_pool_above_threshold() returns true if the pool is above 100%
    #[rule]
    pub fn require_pool_above_threshold_returns_true(e: &Env) {
        let threshold_pc = 10_000_000_000_000_000_000_000_000i128;
        let pool_backstop_data:PoolBackstopData = nondet(); 
        let whole_blnd = pool_backstop_data.blnd/ SCALAR_7;
        let whole_usdc = pool_backstop_data.usdc/ SCALAR_7;
        let saturating_pool_pc = whole_blnd
        .saturating_mul(whole_blnd)
        .saturating_mul(whole_blnd)
        .saturating_mul(whole_blnd)
        .saturating_mul(whole_usdc);

        cvlr_assume!(saturating_pool_pc >= threshold_pc);

        //function call
        let result = require_pool_above_threshold(&pool_backstop_data);

        cvlr_assert!(result == true);
    }

    // require_is_from_pool_factory() passes if if balance == 0 and GHOST_IS_POOL == true
    #[rule]
    pub fn require_is_from_pool_factory_passes_true(e: &Env){
        let address: Address = nondet_address();
        let balance: i128 = cvlr::nondet();
        cvlr_assume!(balance == 0);
        cvlr_assume!(unsafe{GHOST_IS_POOL == true});

        require_is_from_pool_factory(e, &address, balance);

        cvlr_satisfy!(unsafe{GHOST_IS_POOL == true});
    }

    // require_is_from_pool_factory() passes if balance != 0
    #[rule]
    pub fn require_is_from_pool_factory_passes_false(e: &Env){
        let address: Address = nondet_address();
        let balance: i128 = cvlr::nondet();
        cvlr_assume!(balance != 0); 
        cvlr_assume!(unsafe{GHOST_IS_POOL == false});

        require_is_from_pool_factory(e, &address, balance);

        //address is not from the pool but the call passed
        cvlr_satisfy!(unsafe{GHOST_IS_POOL == false}); 
    }

    // require_is_from_pool_factory() reverts if the address is not from the pool factory
    #[rule]
    pub fn require_is_from_pool_factory_fails(e: &Env){
        let address: Address = nondet_address();
        let balance: i128 = cvlr::nondet();
        cvlr_assume!(balance == 0);
        require_is_from_pool_factory(e, &address, balance);
        cvlr_assume!(unsafe{GHOST_IS_POOL == false});
        cvlr_assert!(false); // should never be reached  
    }

    // non_queued_tokens() returns tokens - q4w
    #[rule]
    pub fn non_queued_tokens() {
        let pool_balance: PoolBalance = cvlr::nondet();

        //setup
        cvlr_assume!(pool_balance.shares > 0); // pool has shares
        cvlr_assume!(pool_balance.tokens > 0); //pool has tokens
        cvlr_assume!(pool_balance.q4w <= pool_balance.shares);

        let q4w_tokens = pool_balance.convert_to_tokens(pool_balance.q4w);
     
        let non_queued_return_value = pool_balance.non_queued_tokens();
        let target = pool_balance.tokens - q4w_tokens;
     
        cvlr_assert!(non_queued_return_value == target);
    }

    // convert_to_shares() rounds to floor
    #[rule]
    pub fn convert_to_shares_rounds_down() {
        let tokens: i128 = cvlr::nondet();
        let pool_balance: PoolBalance = cvlr::nondet();
        cvlr_assume!(tokens > 0);
        cvlr_assume!(pool_balance.shares > 0); //pool has shares
        cvlr_assume!(pool_balance.tokens > 0); //pool has tokens

        let target_result = tokens * pool_balance.shares / pool_balance.tokens;

        let shares = pool_balance.convert_to_shares(tokens);
        
        cvlr_assert!(shares == target_result, "shares should be equal to target result");
        cvlr_assert!(pool_balance.shares > 0, "pool has shares");
        cvlr_assert!(pool_balance.tokens > 0, "pool has tokens");
    }

    // convert_to_tokens() rounds to floor
    #[rule]
    pub fn convert_to_tokens_rounds_down() {
        let shares: i128 = cvlr::nondet();
        let pool_balance: PoolBalance = cvlr::nondet();
        cvlr_assume!(pool_balance.shares > 0); //covered by other rule
        cvlr_assume!(shares > 0); //@audit-issue is it checked if shares != 0 every time before convert_to_tokens is called?

        // cvlr_assume!(pool_balance.shares < shares); // pool has shares //@audit-issue it is an issue to pass more shares than the pool has can this be explodet?
        //https://prover.certora.com/output/8418/6ddd2975512844b2b0eb0900abb375a0?anonymousKey=19b1d50e71a0f2d1283c7813420380626c5ac852
        cvlr_assume!(pool_balance.tokens > 0); // pool has tokens
        let target_result = shares * pool_balance.tokens / pool_balance.shares;
        let tokens = pool_balance.convert_to_tokens(shares);
        cvlr_assert!(tokens == target_result);
    }

    // queue_for_withdraw() increases self.q4w by shares and nothing else
    #[rule]
    pub fn queue_for_withdraw_increases(pool_balance: &mut PoolBalance, shares: i128) {
        //setup
        pool_balance_sanity(pool_balance);

        // values before
        let tokens_before = pool_balance.tokens;
        let shares_before = pool_balance.shares;
        let q4w_before = pool_balance.q4w;

        // function call
        pool_balance.queue_for_withdraw(shares);

        // values after
        let tokens_after = pool_balance.tokens;
        let shares_after = pool_balance.shares;
        let q4w_after = pool_balance.q4w;

        //assert
        cvlr_assert!(tokens_after == tokens_before);
        cvlr_assert!(shares_after == shares_before);
        cvlr_assert!(q4w_after == q4w_before + shares);
    }

    // dequeue_q4w() reverts if shares bigger than self.q4w
    #[rule]
    pub fn dequeue_q4w_reverts(e: &Env, pool_balance: &mut PoolBalance, shares: i128) {
        //setup
        pool_balance_sanity(pool_balance);
        cvlr_assume!(shares > pool_balance.q4w); // shares more than q4w

        //function call
        pool_balance.dequeue_q4w(e, shares);

        cvlr_assert!(false);
    }

    // dequeue_q4w() reduces self.q4w by shares and nothing else
    #[rule]
    pub fn dequeue_q4w_reduces(e: &Env, pool_balance: &mut PoolBalance, shares: i128) {
        //setup
        pool_balance_sanity(pool_balance);

        // values before
        let tokens_before = pool_balance.tokens;
        let shares_before = pool_balance.shares;
        let q4w_before = pool_balance.q4w;

        // function call
        pool_balance.dequeue_q4w(e, shares);

        // values after
        let tokens_after = pool_balance.tokens;
        let shares_after = pool_balance.shares;
        let q4w_after = pool_balance.q4w;

        //assert
        cvlr_assert!(tokens_after == tokens_before);
        cvlr_assert!(shares_after == shares_before);
        cvlr_assert!(q4w_after == q4w_before - shares);
    }

    // withdraw() reduces self.tokens by tokens
    #[rule]
    pub fn withdraw_reduces(e: &Env, pool_balance: &mut PoolBalance, tokens: i128, shares: i128) {
        //setup
        pool_balance_sanity(pool_balance);

        // values before
        let tokens_before = pool_balance.tokens;
        let shares_before = pool_balance.shares;
        let q4w_before = pool_balance.q4w;

        // function call
        pool_balance.withdraw(e, tokens, shares);

        // values after
        let tokens_after = pool_balance.tokens;
        let shares_after = pool_balance.shares;
        let q4w_after = pool_balance.q4w;

        //assert
        cvlr_assert!(tokens_after == tokens_before - tokens);
        cvlr_assert!(shares_after == shares_before - shares);
        cvlr_assert!(q4w_after == q4w_before - shares);
    }

    // withdraw() revers if tokens more than self.tokens
    #[rule]
    pub fn withdraw_tokens_more_than_pool(e: &Env, pool_balance: &mut PoolBalance, tokens: i128, shares: i128) {
        //setup
        pool_balance_sanity(pool_balance);
        cvlr_assume!(tokens > pool_balance.tokens); // tokens more than pool

        //function call
        pool_balance.withdraw(e, tokens, shares);

        cvlr_assert!(false);
    }

    // withdraw() reverts if shares more than self.shares
    #[rule]
    pub fn withdraw_shares_more_than_pool(e: &Env, pool_balance: &mut PoolBalance, tokens: i128, shares: i128) {
        //setup
        pool_balance_sanity(pool_balance);
        cvlr_assume!(shares > pool_balance.shares); // shares more than pool

        //function call
        pool_balance.withdraw(e, tokens, shares);

        cvlr_assert!(false);
    }

    // withdraw() reverts if shares more than self.q4w
    #[rule]
    pub fn withdraw_shares_more_than_q4w(e: &Env, pool_balance: &mut PoolBalance, tokens: i128, shares: i128) {
        //setup
        pool_balance_sanity(pool_balance);
        cvlr_assume!(shares > pool_balance.q4w); // shares more than q4w

        //function call
        pool_balance.withdraw(e, tokens, shares);

        cvlr_assert!(false);
    }

    // deposit() increase self.tokens by tokens and self.shares by shares, self.q4w stays the same 
    #[rule]
    pub fn deposit_check(pool_balance: &mut PoolBalance, tokens: i128, shares: i128) {
        //setup
        pool_balance_sanity(pool_balance);

        // values before
        let tokens_before = pool_balance.tokens;
        let shares_before = pool_balance.shares;
        let q4w_before = pool_balance.q4w;

        // function call
        pool_balance.deposit(tokens, shares);

        // values after
        let tokens_after = pool_balance.tokens;
        let shares_after = pool_balance.shares;
        let q4w_after = pool_balance.q4w;
        

        //assert
        cvlr_assert!(tokens_after == tokens_before + tokens);
        cvlr_assert!(shares_after == shares_before + shares);
        cvlr_assert!(q4w_after == q4w_before);
    }

    // convert_to_tokens() returns shares if there are no shares in the pool
    #[rule]
    pub fn convert_to_tokens_no_shares(pool_balance: &mut PoolBalance, shares: i128) {
        cvlr_assume!(pool_balance.shares == 0); // pool has 0 shares
        let tokens = pool_balance.convert_to_tokens(shares);
        cvlr_assert!(tokens == shares);
    }
   
    //convert_to_shares() returns tokens if no shares
    #[rule]
    pub fn convert_to_shares_no_shares(pool_balance: &mut PoolBalance, tokens: i128) {
        cvlr_assume!(pool_balance.shares == 0); // pool has 0 shares
        let shares = pool_balance.convert_to_shares(tokens);
        cvlr_assert!(shares == tokens);

    }


//--------------------------- OLD RULES START ---------------------------

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

//--------------------------- OLD RULES END ---------------------------