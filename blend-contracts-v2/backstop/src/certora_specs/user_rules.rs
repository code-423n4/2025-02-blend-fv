use cvlr::{cvlr_assert, cvlr_assume};
use cvlr_soroban_derive::rule;

use crate::backstop::UserBalance;

// deposit should increase user shares
#[rule]
pub fn add_shares_increases_user_shares(user_balance: &mut UserBalance, shares: i128) {
    cvlr_assume!(shares >= 0);
    let user_shares_before = user_balance.shares;
    user_balance.add_shares(shares);
    let user_shares_after = user_balance.shares;
    cvlr_assert!(user_shares_after == user_shares_before + shares);
}
