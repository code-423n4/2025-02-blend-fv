#![no_std]

use soroban_sdk::{contract, contractimpl, token, Address, Env};

#[contract]
pub struct FlashLoanReceiverModifiedERC3156;

#[contractimpl]
impl FlashLoanReceiverModifiedERC3156 {
    pub fn exec_op(env: Env, caller: Address, token: Address, amount: i128, _fee: i128) {
        // require the caller to authorize the invocation
        caller.require_auth();

        // perform operations here
        // ...

        // Test - return the amount to caller so they can repay the flash loan.
        token::Client::new(&env, &token).transfer(
            &env.current_contract_address(),
            &caller,
            &amount,
        );
    }
}
