// #![allow(unused)]
// use soroban_sdk::{Env, Address};

// use crate::{storage::BackstopEmissionData, PoolBalance, UserBalance};

// pub fn update_emissions(
//     e: &Env,
//     pool_id: &Address,
//     pool_balance: &PoolBalance,
//     user_id: &Address,
//     user_balance: &UserBalance,
// ) {
//     update_rz_emis_data(e, pool_id, false);
//     if let Some(emis_data) = update_emission_data(e, pool_id, pool_balance) {
//         update_user_emissions(e, pool_id, user_id, &emis_data, user_balance, false);
//     }
// }