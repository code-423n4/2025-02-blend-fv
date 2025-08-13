// use cvlr::nondet;
// use soroban_sdk::{Address, String};
// use blend_contract_sdk::backstop::UserBalance;

// use crate::{backstop::GhostMap, PoolBalance};

pub(crate) mod summaries;
pub(crate) mod mocks;

pub(crate) mod pool_rules;
pub(crate) mod withdraw_rules;
pub(crate) mod fund_management_rules;
pub(crate) mod deposit_rules;
pub(crate) mod user_rules;

//--------------- added for ghost START-------------------
// #[cfg(feature = "certora")]
pub(crate) static mut GHOST_IS_POOL: bool = true; 
pub(crate) static mut GHOST_UPDATE_EMISSIONS_CALLED: bool = false; 
// pub(crate) static mut GHOST_UPDATE_EMISSIONS_POOL: String = String::default(); //compare with address.to_string() 
// pub(crate) static mut GHOST_UPDATE_EMISSIONS_POOL_BALANCE: PoolBalance ; 
// pub(crate) static mut GHOST_UPDATE_EMISSIONS_FROM: Address; 
// pub(crate) static mut GHOST_UPDATE_EMISSIONS_USER_BALANCE: UserBalance = serBalance::env_default(); 
pub(crate) static mut GHOST_TOTAL_SUPPLY: i128 = 0; 
// ghosts for token transfer
pub(crate) static mut GHOST_FROM_BALANCE: i128 = 0;
pub(crate) static mut GHOST_TO_BALANCE: i128 = 0;
pub(crate) static mut GHOST_ALLOWANCE: i128 = 0;

//ghost for load_pool_backstop
pub(crate) static mut GHOST_POOL_TOTAL_SUPPLY: i128 = 0;
pub(crate) static mut GHOST_POOL_BLND_BALANCE: i128 = 0;
pub(crate) static mut GHOST_POOL_USDC_BALANCE: i128 = 0;
pub(crate) static mut GHOST_Q4W_PCT: i128 = 0;
pub(crate) static mut GHOST_USDC_PER_TOKEN: i128 = 0;
pub(crate) static mut GHOST_BLND_PER_TOKEN: i128 = 0;
pub(crate) static mut GHOST_BLND: i128 = 0;
pub(crate) static mut GHOST_USDC: i128 = 0;

//ghost for withdraw shares to token 
pub(crate) static mut GHOST_TO_RETURN: i128 = 0;



//--------------- added for ghost END-------------------
