pub(crate) mod mocks;
pub(crate) mod base;
pub(crate) mod summaries;
pub(crate) mod sanity;
pub(crate) mod valid_state;
pub(crate) mod state_trans;
pub(crate) mod high_level;
pub(crate) mod integrity_balance;
pub(crate) mod integrity_emission;
pub(crate) mod integrity_token;
pub(crate) mod isolation;

use crate::backstop::{self, PoolBackstopData, PoolBalance, UserBalance};
use crate::certora_specs::base::{Call, log_state::log_state_details, clear_upper_bits};
use crate::{init_verification, invariant_rule, make_callable, parametric_rule, pass_arg};
use cvlr::{cvlr_assert, cvlr_assume};
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env, unwrap::UnwrapOptimized};
use crate::certora_specs::sanity::sanity;
use crate::certora_specs::valid_state::{
    valid_state_pool_user,
    valid_state_nonnegative_pb_shares,
    valid_state_nonnegative_pb_tokens,
    valid_state_nonnegative_pb_q4w,
    valid_state_nonnegative_ub_shares,
    valid_state_nonnegative_ub_q4w_amount,
    valid_state_pb_q4w_leq_shares,
    valid_state_ub_shares_plus_q4w_sum_eq_pb_shares,
    valid_state_ub_q4w_expiration,
    valid_state_ub_q4w_sum_eq_pb_q4w,
    valid_state_ub_q4w_exp_implies_amount,
    valid_state_user_not_pool,
    valid_state_pool_from_factory,
};
use crate::certora_specs::state_trans::{
    state_trans_pb_shares_tokens_directional_change,
    state_trans_pb_q4w_consistency,
    state_trans_ub_shares_increase_consistency,
    state_trans_ub_shares_decrease_consistency,
    state_trans_ub_q4w_amount_consistency,
};
use crate::certora_specs::isolation::{
    isolation_pool,
    isolation_user,
};
#[cfg(feature = "certora_storage_ghost")] 
use crate::certora_specs::summaries::storage;
#[cfg(not(feature = "certora_storage_ghost"))]
use crate::storage;

pub const FV_MAX_Q4W_VEC_LEN: u32 = 1;

// Helpers for external functions
make_callable!(backstop, execute_deposit, from: Address, pool_address: Address, amount: i128);
make_callable!(backstop, execute_donate, from: Address, pool_address: Address, amount: i128);
make_callable!(backstop, execute_draw, pool_address: Address, amount: i128, to: Address);
make_callable!(backstop, execute_dequeue_withdrawal, from: Address, pool_address: Address, amount: i128);
make_callable!(backstop, execute_queue_withdrawal, from: Address, pool_address: Address, amount: i128);
make_callable!(backstop, execute_withdraw, from: Address, pool_address: Address, amount: i128);
make_callable!(backstop, load_pool_backstop_data, address: Address);
make_callable!(backstop, require_is_from_pool_factory, address: Address, balance: i128);
make_callable!(backstop, require_pool_above_threshold, no_env, pool_backstop_data: PoolBackstopData);

// Sanity parametric rule
parametric_rule!(sanity);

// Valid state parametric rules in invariant style
invariant_rule!(valid_state_nonnegative_pb_shares);
invariant_rule!(valid_state_nonnegative_pb_tokens);
invariant_rule!(valid_state_nonnegative_pb_q4w);
invariant_rule!(valid_state_nonnegative_ub_shares);
invariant_rule!(valid_state_nonnegative_ub_q4w_amount);
invariant_rule!(valid_state_ub_shares_plus_q4w_sum_eq_pb_shares);
invariant_rule!(valid_state_ub_q4w_expiration);
invariant_rule!(valid_state_ub_q4w_sum_eq_pb_q4w);
invariant_rule!(valid_state_ub_q4w_exp_implies_amount);
invariant_rule!(valid_state_pb_q4w_leq_shares);
invariant_rule!(valid_state_user_not_pool);
invariant_rule!(valid_state_pool_from_factory);

// State transition parametric rules
parametric_rule!(state_trans_pb_shares_tokens_directional_change);
parametric_rule!(state_trans_pb_q4w_consistency);
parametric_rule!(state_trans_ub_shares_increase_consistency);
parametric_rule!(state_trans_ub_shares_decrease_consistency);
parametric_rule!(state_trans_ub_q4w_amount_consistency);

// Parametric rules for isolation test
parametric_rule!(isolation_pool);
parametric_rule!(isolation_user);
