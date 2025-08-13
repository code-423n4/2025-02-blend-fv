
## deposit.rs Mutations

**deposit_0:** 
Mutation: Skip update of pool_balance in execute_deposit() so the pool appears to have less liquidity and fewer shares than it actually does. 

**deposit_1:** 
Mutation: Skip update of user_balance in execute_deposit(). 

**deposit_2:** 
Mutation: Remove error check in execute_deposit() for minting zero or negative shares. 

**deposit_3:** 
Mutation: Remove input validation in execute_deposit() for negative deposits.

## fund_management.rs Mutations

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

## pool.rs Mutations

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

## user.rs Mutations

**user_0:** 
Mutation: Fail to increase user's balance in add_shares(). 

**user_1:** 
Mutation: Increase (instead of decrease) user's balance inqueue_shares_for_withdrawal(). 

**user_2:** 
Mutation: Fail to count queued shares in dequeue_shares() when queued amount is larger than withdrawal, creating infinite loop (denial of service). 

**user_3:** 
Mutation: Fail to dequeue shares in withdraw_shares() if first queued amount is smaller than withdrawal. 

## withdraw.rs Mutations

**withdraw_0:** 
Mutation: Fail to update stored user balance in execute_dequeue_withdrawal(). 

**withdraw_1:** 
Mutation: Fail to queue user shares for withdrawal in execute_queue_withdrawal(). 

**withdraw_2:** 
Mutation: Fail to increase user's share balance in execute_dequeue_withdrawal. 

**withdraw_3:** 
Mutation: Remove validation in execute_withdraw() that prevents withdrawing a 0 token amount. 


