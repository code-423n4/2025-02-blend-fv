# Certora Contest Report: Blend v2

## Table of contents

- [**Overview**](#overview)
    - [**About Certora**](#about-certora)
    - [**Summary**](#summary)
- [**Mutations**](#mutations)
- [**Notable Properties**](#notable-properties)
    - [**Caught Mutations in Contract: deposit**](#caught-mutations-in-contract-deposit)
    - [**Caught Mutations in Contract: fund_management**](#caught-mutations-in-contract-fund_management)
    - [**Caught Mutations in Contract: pool**](#caught-mutations-in-contract-pool)
    - [**Caught Mutations in Contract: user**](#caught-mutations-in-contract-user)
    - [**Caught Mutations in Contract: withdraw**](#caught-mutations-in-contract-withdraw)
- [**Disclaimer**](#disclaimer)

# Overview

## About Certora

Certora is a Web3 security company that provides industry-leading formal verification tools and smart contract audits. Certora’s flagship security product, Certora Prover, is a unique SaaS product which locates hard-to-find bugs in smart contracts or mathematically proves their absence.

A formal verification contest is a competition where members of the community mathematically validate the accuracy of smart contracts, in return for a reward offered by the sponsor based on the participants' findings and property coverage.

In the formal verification contest detailed here, contenders formally verified Blend v2 smart contracts. This formal verification contest was held from February 24, 2025 until March 17, 2025 as part of the [audit hosted by Code4rena](https://code4rena.com/audits/2025-02-blend-v2-audit-certora-formal-verification).

## Summary 

Code "mutations" were introduced to evaluate the quality of the specifications written by the contest participants. The mutations are described below and were made available at the end of the contest in the [updated mutations directory in the contest repository here](https://github.com/code-423n4/2025-02-blend-fv/tree/main/blend-contracts-v2/backstop/mutations) along with a description of each mutation.

There were 229 properties submitted by 21 participants that successfully caught mutations. Some of those properties are included below in this report as examples of high-quality properties. Additionally, the top specifications have been added to the [contest repo](https://github.com/code-423n4/2025-02-blend-fv/tree/main/Example_Submissions). You can find the final results for the competition [here](https://docs.google.com/spreadsheets/d/1g0_RmhGVQJeQvn5vrAfhew-0hjq0iT0TO8OgsYPmfAg/edit?gid=1970712821#gid=1970712821).

# Mutations

## Deposit Mutations
**deposit_0:** 
Mutation: Skip update of pool_balance in execute_deposit() so the pool appears to have less liquidity and fewer shares than it actually does. 

**deposit_1:** 
Mutation: Skip update of user_balance in execute_deposit(). 

**deposit_2:** 
Mutation: Remove error check in execute_deposit() for minting zero or negative shares. 

**deposit_3:** 
Mutation: Remove input validation in execute_deposit() for negative deposits.

## Fund Management Mutations
**fund_management_0:** 
Mutation: Fail to reduce pool balance in execute_draw(). 

**fund_management_1:** 
Mutation: Remove input validation in execute_draw() for negative withdrawals. 

**fund_management_2:** 
Mutation: Remove input validation in execute_donate() for negative donations. 

**fund_management_3:** 
Mutation: Fail to increase pool balance in execute_donate(). 

**fund_management_4:** 
Mutation: Increase pool balance by reduced amount in execute_donate(). 

## Pool Mutations
**pool_0:** 
Mutation: Skip reduction of token balance in withdraw(). 

**pool_1:** 
Mutation: Skip reduction of share balance in withdraw(). 

**pool_2:** 
Mutation: Skip reduction of queued shares in withdraw(). 

**pool_3:** 
Mutation: Skip increase of token balance in deposit(). 

**pool_4:** 
Mutation: Decrease (instead of increase) queued shares in queue_for_withdraw(). 

## User Mutations
**user_0:** 
Mutation: Fail to increase user's balance in add_shares(). 

**user_1:** 
Mutation: Increase (instead of decrease) user's balance inqueue_shares_for_withdrawal(). 

**user_2:** 
Mutation: Fail to count queued shares in dequeue_shares() when queued amount is larger than withdrawal, creating infinite loop (denial of service). 

**user_3:** 
Mutation: Fail to dequeue shares in withdraw_shares() if first queued amount is smaller than withdrawal. 

## Withdraw Mutations
**withdraw_0:** 
Mutation: Fail to update stored user balance in execute_dequeue_withdrawal(). 

**withdraw_1:** 
Mutation: Fail to queue user shares for withdrawal in execute_queue_withdrawal(). 

**withdraw_2:** 
Mutation: Fail to increase user's share balance in execute_dequeue_withdrawal. 

**withdraw_3:** 
Mutation: Remove validation in execute_withdraw() that prevents withdrawing a 0 token amount. 

# Notable Properties

## Caught Mutations in Contract: deposit

### Ensure Negative Deposit Fails

*Author:* PositiveSecurity

Mutation(s) caught: deposit_2, deposit_3

```
pub fn execute_deposit_negative_amount_fails(e: &Env, from: Address, pool_address: Address, amount: i128) {
    cvlr_assume!(amount <= 0);
    execute_deposit(&e, &from, &pool_address, amount);
    cvlr_assert!(false);
}
```

### Deposit Correctly Affects Share and Token Balances

*Author:* X0rD3v1L

Mutation(s) caught: deposit_0, deposit_1

```
pub fn execute_deposit_correctness(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    cvlr_assume!(amount > 0);
    cvlr_assume!(from != pool_address);
    cvlr_assume!(from != &e.current_contract_address());

    let pool_balance_before = storage::get_pool_balance(e, pool_address);
    let user_balance_before = storage::get_user_balance(e, pool_address, from);
    
    let user_shares_before = user_balance_before.shares;
    let pool_shares_before = pool_balance_before.shares;
    let pool_tokens_before = pool_balance_before.tokens; // Assuming pool stores token balance
    
    let minted_shares = execute_deposit(e, from, pool_address, amount);

    let pool_balance_after = storage::get_pool_balance(e, pool_address);
    let user_balance_after = storage::get_user_balance(e, pool_address, from);
    
    let user_shares_after = user_balance_after.shares;
    let pool_shares_after = pool_balance_after.shares;
    let pool_tokens_after = pool_balance_after.tokens;

    cvlr_assert!(minted_shares > 0);
    cvlr_assert!(user_shares_after == user_shares_before + minted_shares);
    cvlr_assert!(pool_shares_after == pool_shares_before + minted_shares);
    cvlr_assert!(pool_tokens_after == pool_tokens_before + amount);
}
```

## Caught Mutations in Contract: fund_management

### Draw After Equal Donate Yields Original Pool Token Balance

*Author:* kind0Dev

Mutation(s) caught: fund_management_0, fund_management_3, fund_management_4

```
pub fn draw_after_donation_neutral(e: Env, from: Address, pool_address: Address, donate_amount: i128, to: &Address) {
    cvlr_assume!(donate_amount > 0, "Donation amount must be positive");
    let pool_balance = storage::get_pool_balance(&e, &pool_address);


    let initial_pool_tokens = pool_balance.tokens;

    execute_donate(&e, &from, &pool_address, donate_amount);
    execute_draw(&e, &pool_address, donate_amount, &to);
    let pool_balance_after = storage::get_pool_balance(&e, &pool_address);


    cvlr_assert!(
        pool_balance_after.tokens == initial_pool_tokens,
        "Donation followed by immediate draw should leave pool token balance unchanged"
    );
}
```

### Donate Correctly Increases Pool Token Balance

*Author:* Zac369

Mutation(s) caught: fund_management_3, fund_management_4

```
pub fn fund_deposit(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    let pool_balance_before = storage::get_pool_balance(e, pool_address);
    let pool_tokens_before = pool_balance_before.tokens;
    execute_donate(e, from, pool_address, amount);
    let pool_balance_after = storage::get_pool_balance(e, pool_address);
    let pool_tokens_after = pool_balance_after.tokens;
    cvlr_assert!(
        pool_tokens_after == pool_tokens_before + amount
    );
}
```

## Caught Mutations in Contract: pool

### Withdraw After Equal Deposit and Enqueue Yields Original Pool Balance

*Author:* LSHFGJ

Mutation(s) caught: pool_0, pool_1, pool_2, pool_3, pool_4

```
pub fn deposit_withdraw_no_change(e: &Env, pool_balance: &mut PoolBalance, tokens: i128, shares: i128) {
    let pool_balance_before = pool_balance.clone();
    pool_balance.deposit(tokens, shares);
    pool_balance.queue_for_withdraw(shares);
    pool_balance.withdraw(e, tokens, shares);
    let pool_balance_after = pool_balance.clone();
    cvlr_assert!(
        pool_balance_before.tokens == pool_balance_after.tokens &&
        pool_balance_before.shares == pool_balance_after.shares &&
        pool_balance_before.q4w == pool_balance_after.q4w
    );
}
```

### Withdraw Requires Non-empty Queue and Positive Balance

*Author:* jraynaldi3

Mutation(s) caught: pool_0, pool_1, pool_2

```
pub fn withdraw_positive_value(e:&Env, tokens:i128, shares:i128){
    let mut pool_balance: PoolBalance = cvlr::nondet();

    pool_balance.withdraw(e, tokens, shares);

    cvlr_assert!(
        pool_balance.q4w >= 0
        && pool_balance.shares >= 0
        && pool_balance.tokens >= 0
    );
}
```

### Withdraw after Equal Deposit Yields Original Pool Balance

*Author:* Zac369

Mutation(s) caught: pool_0, pool_1, pool_3

```
pub fn deposit_withdraw_equal(e: &Env, pool_balance: &mut PoolBalance, tokens: i128, shares: i128) {
    let pool_tokens_before = pool_balance.tokens;
    let pool_shares_before = pool_balance.shares;

    pool_balance.deposit(tokens, shares);
    pool_balance.withdraw(e, tokens, shares);

    let pool_tokens_after = pool_balance.tokens;
    let pool_shares_after = pool_balance.shares;

    cvlr_assert!(pool_tokens_after == pool_tokens_before);
    cvlr_assert!(pool_shares_after == pool_shares_before);
}
```

## Caught Mutations in Contract: user

### Queueing Shares for Withdrawal Transfers Shares Correctly

*Author:* kind0Dev

Mutation(s) caught: pool_4, user_1, withdraw_1

```
pub fn queue_withdraw_atomicity(from: Address, e: Env, pool_address: Address, queued_shares: i128,
) {
    cvlr_assume!(queued_shares > 0, "Rule relevant for positive queue amounts");
    let user_balance: UserBalance = storage::get_user_balance(&e, &pool_address, &from);
    let pool_balance: PoolBalance = storage::get_pool_balance(&e, &pool_address);

    let user_share_before = user_balance.shares;
    let pool_q4w_share_before = pool_balance.q4w;

    execute_queue_withdrawal(&e, &from, &pool_address, queued_shares);

    let user_balance_new: UserBalance = storage::get_user_balance(&e, &pool_address, &from);
    let pool_balance_new: PoolBalance = storage::get_pool_balance(&e, &pool_address);

    let user_shares_decrease = user_share_before - user_balance_new.shares;
    let pool_q4w_increase = pool_balance_new.q4w - pool_q4w_share_before;

    cvlr_assert!(user_shares_decrease == pool_q4w_increase, "Atomicity: Queuing should reduce user shares by the queued amount.");
}
```

### Withdrawal Correctly Updates Queued Shares

*Author:* BenRai1

Mutation(s) caught: user_3

```
pub fn user_withdraw_shares_rest_pushed_to_front(e: &Env) {
    let mut user_balance: UserBalance =cvlr::nondet();
    let shares: i128 = cvlr::nondet();
    cvlr_assume!(shares >= 0);
    let user_q4w_before = user_balance.q4w.clone();
    cvlr_assume!(user_q4w_before.len() == 3);
    
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
```

## Caught Mutations in Contract: withdraw

### Dequeue Shares Correctly Updates User Balance, Other Users Unaffected

*Author:* BenRai1

Mutation(s) caught: withdraw_0, withdraw_2

```
pub fn dequeue_increase_shares(e: &Env) {
    let from: Address = nondet_address();
    let pool_address: Address = nondet_address();
    let other_user: Address = nondet_address();
    let shares:i128 = cvlr::nondet();
    cvlr_assume!(from != other_user);

    //values before
    let user_shares_before = storage::get_user_balance(e, &pool_address, &from).shares;
    let other_user_shares_before = storage::get_user_balance(e, &pool_address, &other_user).shares;
    
    execute_dequeue_withdrawal(e, &from, &pool_address, shares);

    //values after
    let user_shares_after = storage::get_user_balance(e, &pool_address, &from).shares;
    let other_user_shares_after = storage::get_user_balance(e, &pool_address, &other_user).shares;

    cvlr_assert!(user_shares_after == user_shares_before + shares);
    cvlr_assert!(other_user_shares_after == other_user_shares_before);           
}
```

# Disclaimer

The Certora Prover takes a contract and a specification as input and formally proves that the contract satisfies the specification in all scenarios. Notably, the guarantees of the Certora Prover are scoped to the provided specification and the Certora Prover does not check any cases not covered by the specification. 

Certora does not provide a warranty of any kind, explicit or implied. The contents of this report should not be construed as a complete guarantee that the contract is secure in all dimensions. In no event shall Certora or any of its employees or community participants be liable for any claim, damages, or other liability, whether in an action of contract, tort, or otherwise, arising from, out of, or in connection with the results reported here. All smart contract software should be used at the sole risk and responsibility of users.

