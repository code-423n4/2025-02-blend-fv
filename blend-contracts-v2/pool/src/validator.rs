use soroban_sdk::{panic_with_error, Env};

use crate::errors::PoolError;

/// Require that an incoming amount is not negative
///
/// ### Arguments
/// * `amount` - The amount to check
///
/// ### Panics
/// If the number is negative
pub fn require_nonnegative(e: &Env, amount: &i128) {
    if amount.is_negative() {
        panic_with_error!(e, PoolError::NegativeAmountError);
    }
}
