use cast::i128;
use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::{contracttype, panic_with_error, Address, Env};

use crate::{
    constants::{SCALAR_12, SCALAR_7},
    errors::PoolError,
    pool::actions::RequestType,
    storage::{self, PoolConfig, ReserveConfig, ReserveData},
};

use super::interest::calc_accrual;

#[derive(Clone, Debug)]
#[contracttype]
pub struct Reserve {
    pub asset: Address,        // the underlying asset address
    pub config: ReserveConfig, // the reserve configuration
    pub data: ReserveData,     // the reserve data
    pub scalar: i128,
}

impl Reserve {
    /// Load a Reserve from the ledger and update to the current ledger timestamp.
    ///
    /// **NOTE**: This function is not cached, and should be called from the Pool.
    ///
    /// ### Arguments
    /// * pool_config - The pool configuration
    /// * asset - The address of the underlying asset
    ///
    /// ### Panics
    /// Panics if the asset is not supported, if emissions cannot be updated, or if the reserve
    /// cannot be updated to the current ledger timestamp.
    pub fn load(e: &Env, pool_config: &PoolConfig, asset: &Address) -> Reserve {
        let reserve_config = storage::get_res_config(e, asset);
        let reserve_data = storage::get_res_data(e, asset);
        let mut reserve = Reserve {
            asset: asset.clone(),
            scalar: 10i128.pow(reserve_config.decimals),
            config: reserve_config,
            data: reserve_data,
        };

        // short circuit if the reserve has already been updated this ledger
        if e.ledger().timestamp() == reserve.data.last_time {
            return reserve;
        }

        if reserve.data.b_supply == 0 {
            reserve.data.last_time = e.ledger().timestamp();
            return reserve;
        }

        let cur_util = reserve.utilization(e);
        if cur_util == 0 {
            // if there are no assets borrowed, we don't need to update the reserve
            reserve.data.last_time = e.ledger().timestamp();
            return reserve;
        }

        let (loan_accrual, new_ir_mod) = calc_accrual(
            e,
            &reserve.config,
            cur_util,
            reserve.data.ir_mod,
            reserve.data.last_time,
        );
        reserve.data.ir_mod = new_ir_mod;

        let pre_update_liabilities = reserve.total_liabilities(e);
        reserve.data.d_rate = loan_accrual.fixed_mul_ceil(e, &reserve.data.d_rate, &SCALAR_12);
        let accrued_interest = reserve.total_liabilities(e) - pre_update_liabilities;

        reserve.accrue(e, pool_config.bstop_rate, accrued_interest);

        reserve.data.last_time = e.ledger().timestamp();
        reserve
    }

    /// Store the updated reserve to the ledger.
    pub fn store(&self, e: &Env) {
        storage::set_res_data(e, &self.asset, &self.data);
    }

    /// Accrue tokens to the reserve supply. This issues any `backstop_credit` required and updates the reserve's bRate to account for the additional tokens.
    ///
    /// ### Arguments
    /// * bstop_rate - The backstop take rate for the pool
    /// * accrued - The amount of additional underlying tokens
    fn accrue(&mut self, e: &Env, bstop_rate: u32, accrued: i128) {
        let pre_update_supply = self.total_supply(e);

        if accrued > 0 {
            // credit the backstop underlying from the accrued interest based on the backstop rate
            // update the accrued interest to reflect the amount the pool accrued
            let mut new_backstop_credit: i128 = 0;
            if bstop_rate > 0 {
                new_backstop_credit = accrued.fixed_mul_floor(e, &i128(bstop_rate), &SCALAR_7);
                self.data.backstop_credit += new_backstop_credit;
            }
            self.data.b_rate = (pre_update_supply + accrued - new_backstop_credit).fixed_div_floor(
                e,
                &self.data.b_supply,
                &SCALAR_12,
            );
        }
    }

    /// Fetch the current utilization rate for the reserve normalized to 7 decimals
    pub fn utilization(&self, e: &Env) -> i128 {
        self.total_liabilities(e)
            .fixed_div_ceil(e, &self.total_supply(e), &SCALAR_7)
    }

    /// Require that the utilization rate is below the maximum allowed, or panic.
    pub fn require_utilization_below_max(&self, e: &Env) {
        if self.utilization(e) > i128(self.config.max_util) {
            panic_with_error!(e, PoolError::InvalidUtilRate)
        }
    }

    /// Check the action is allowed according to the reserve status, or panic.
    ///
    /// ### Arguments
    /// * `action_type` - The type of action being performed
    pub fn require_action_allowed(&self, e: &Env, action_type: u32) {
        // disable borrowing or auction cancellation for any non-active pool and disable supplying for any frozen pool
        if !self.config.enabled {
            if action_type == RequestType::Supply as u32
                || action_type == RequestType::SupplyCollateral as u32
                || action_type == RequestType::Borrow as u32
            {
                panic_with_error!(e, PoolError::ReserveDisabled);
            }
        }
    }

    /// Fetch the total liabilities for the reserve in underlying tokens
    pub fn total_liabilities(&self, e: &Env) -> i128 {
        self.to_asset_from_d_token(e, self.data.d_supply)
    }

    /// Fetch the total supply for the reserve in underlying tokens
    pub fn total_supply(&self, e: &Env) -> i128 {
        self.to_asset_from_b_token(e, self.data.b_supply)
    }

    /********** Conversion Functions **********/

    /// Convert d_tokens to the corresponding asset value
    ///
    /// ### Arguments
    /// * `d_tokens` - The amount of tokens to convert
    pub fn to_asset_from_d_token(&self, e: &Env, d_tokens: i128) -> i128 {
        d_tokens.fixed_mul_ceil(e, &self.data.d_rate, &SCALAR_12)
    }

    /// Convert b_tokens to the corresponding asset value
    ///
    /// ### Arguments
    /// * `b_tokens` - The amount of tokens to convert
    pub fn to_asset_from_b_token(&self, e: &Env, b_tokens: i128) -> i128 {
        b_tokens.fixed_mul_floor(e, &self.data.b_rate, &SCALAR_12)
    }

    /// Convert d_tokens to their corresponding effective asset value. This
    /// takes into account the liability factor.
    ///
    /// ### Arguments
    /// * `d_tokens` - The amount of tokens to convert
    pub fn to_effective_asset_from_d_token(&self, e: &Env, d_tokens: i128) -> i128 {
        let assets = self.to_asset_from_d_token(e, d_tokens);
        assets.fixed_div_ceil(e, &i128(self.config.l_factor), &SCALAR_7)
    }

    /// Convert b_tokens to the corresponding effective asset value. This
    /// takes into account the collateral factor.
    ///
    /// ### Arguments
    /// * `b_tokens` - The amount of tokens to convert
    pub fn to_effective_asset_from_b_token(&self, e: &Env, b_tokens: i128) -> i128 {
        let assets = self.to_asset_from_b_token(e, b_tokens);
        assets.fixed_mul_floor(e, &i128(self.config.c_factor), &SCALAR_7)
    }

    /// Convert asset tokens to the corresponding d token value - rounding up
    ///
    /// ### Arguments
    /// * `amount` - The amount of tokens to convert
    pub fn to_d_token_up(&self, e: &Env, amount: i128) -> i128 {
        amount.fixed_div_ceil(e, &self.data.d_rate, &SCALAR_12)
    }

    /// Convert asset tokens to the corresponding d token value - rounding down
    ///
    /// ### Arguments
    /// * `amount` - The amount of tokens to convert
    pub fn to_d_token_down(&self, e: &Env, amount: i128) -> i128 {
        amount.fixed_div_floor(e, &self.data.d_rate, &SCALAR_12)
    }

    /// Convert asset tokens to the corresponding b token value - round up
    ///
    /// ### Arguments
    /// * `amount` - The amount of tokens to convert
    pub fn to_b_token_up(&self, e: &Env, amount: i128) -> i128 {
        amount.fixed_div_ceil(e, &self.data.b_rate, &SCALAR_12)
    }

    /// Convert asset tokens to the corresponding b token value - round down
    ///
    /// ### Arguments
    /// * `amount` - The amount of tokens to convert
    pub fn to_b_token_down(&self, e: &Env, amount: i128) -> i128 {
        amount.fixed_div_floor(e, &self.data.b_rate, &SCALAR_12)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutils;
    use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};

    #[test]
    fn test_load_reserve() {
        let e = Env::default();
        e.mock_all_auths();

        e.ledger().set(LedgerInfo {
            timestamp: 123456 * 5,
            protocol_version: 22,
            sequence_number: 123456,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let oracle = Address::generate(&e);

        let (underlying, _) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, mut reserve_data) = testutils::default_reserve_meta();
        reserve_data.d_rate = 1_345_678_123_000;
        reserve_data.b_rate = 1_123_456_789_000;
        reserve_data.d_supply = 65_0000000;
        reserve_data.b_supply = 99_0000000;
        testutils::create_reserve(&e, &pool, &underlying, &reserve_config, &reserve_data);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_2000000,
            status: 0,
            max_positions: 5,
        };
        e.as_contract(&pool, || {
            storage::set_pool_config(&e, &pool_config);
            let reserve = Reserve::load(&e, &pool_config, &underlying);

            // (accrual: 1_002_957_375_248, util: .7864353)
            assert_eq!(reserve.data.d_rate, 1_349_657_798_173);
            assert_eq!(reserve.data.b_rate, 1_125_547_124_242);
            assert_eq!(reserve.data.ir_mod, 1_0449815);
            assert_eq!(reserve.data.d_supply, 65_0000000);
            assert_eq!(reserve.data.b_supply, 99_0000000);
            assert_eq!(reserve.data.backstop_credit, 0_0517357);
            assert_eq!(reserve.data.last_time, 617280);
        });
    }

    #[test]
    fn test_load_reserve_accrues_b_rate() {
        let e = Env::default();
        e.mock_all_auths();

        e.ledger().set(LedgerInfo {
            timestamp: 1000,
            protocol_version: 22,
            sequence_number: 123456,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let oracle = Address::generate(&e);

        // setup load reserve with minimal interest gained (5s / low util / high supply)
        // to validate b/d rate is still safely accrued
        let (underlying, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config, mut reserve_data) = testutils::default_reserve_meta();
        reserve_config.decimals = 18;
        let scalar = 10i128.pow(reserve_config.decimals);
        reserve_data.d_rate = 1_500_000_000_000;
        reserve_data.b_rate = 1_300_000_000_000;
        reserve_data.ir_mod = SCALAR_7;
        reserve_data.d_supply = 100_000_000 * scalar;
        reserve_data.b_supply = 10_000_000_000 * scalar;
        reserve_data.last_time = 995;
        testutils::create_reserve(&e, &pool, &underlying, &reserve_config, &reserve_data);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_2000000,
            status: 0,
            max_positions: 5,
        };
        e.as_contract(&pool, || {
            storage::set_pool_config(&e, &pool_config);
            let reserve = Reserve::load(&e, &pool_config, &underlying);

            // validate that b and d rates are updated
            assert_eq!(reserve.data.last_time, 1000);
            assert_eq!(reserve.data.b_rate, 1_300_000_000_020);
            assert_eq!(reserve.data.d_rate, 1_500_000_002_562);
            assert_eq!(reserve.data.ir_mod, 9999927);
            assert_eq!(reserve.data.backstop_credit, 0_051240000_000000000);
        });
    }

    #[test]
    fn test_load_reserve_zero_supply() {
        let e = Env::default();
        e.mock_all_auths();

        e.ledger().set(LedgerInfo {
            timestamp: 123456 * 5,
            protocol_version: 22,
            sequence_number: 123456,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let oracle = Address::generate(&e);

        let (underlying, _) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, mut reserve_data) = testutils::default_reserve_meta();
        reserve_data.d_rate = 0;
        reserve_data.b_rate = 0;
        reserve_data.d_supply = 0;
        reserve_data.b_supply = 0;
        testutils::create_reserve(&e, &pool, &underlying, &reserve_config, &reserve_data);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_2000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool, || {
            storage::set_pool_config(&e, &pool_config);
            let reserve = Reserve::load(&e, &pool_config, &underlying);

            assert_eq!(reserve.data.d_rate, 0);
            assert_eq!(reserve.data.b_rate, 0);
            assert_eq!(reserve.data.ir_mod, 10000000);
            assert_eq!(reserve.data.d_supply, 0);
            assert_eq!(reserve.data.b_supply, 0);
            assert_eq!(reserve.data.backstop_credit, 0);
            assert_eq!(reserve.data.last_time, 617280);
        });
    }

    #[test]
    fn test_load_reserve_zero_util() {
        let e = Env::default();
        e.mock_all_auths();

        e.ledger().set(LedgerInfo {
            timestamp: 123456 * 5,
            protocol_version: 22,
            sequence_number: 123456,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let oracle = Address::generate(&e);

        let (underlying, _) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, mut reserve_data) = testutils::default_reserve_meta();
        reserve_data.d_rate = 0;
        reserve_data.d_supply = 0;
        testutils::create_reserve(&e, &pool, &underlying, &reserve_config, &reserve_data);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_2000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool, || {
            storage::set_pool_config(&e, &pool_config);
            let reserve = Reserve::load(&e, &pool_config, &underlying);

            assert_eq!(reserve.data.d_rate, 0);
            assert_eq!(reserve.data.b_rate, reserve_data.b_rate);
            assert_eq!(reserve.data.ir_mod, reserve_data.ir_mod);
            assert_eq!(reserve.data.d_supply, 0);
            assert_eq!(reserve.data.b_supply, reserve_data.b_supply);
            assert_eq!(reserve.data.backstop_credit, 0);
            assert_eq!(reserve.data.last_time, 617280);
        });
    }

    #[test]
    fn test_load_reserve_zero_bstop_rate() {
        let e = Env::default();
        e.mock_all_auths();

        e.ledger().set(LedgerInfo {
            timestamp: 123456 * 5,
            protocol_version: 22,
            sequence_number: 123456,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let oracle = Address::generate(&e);

        let (underlying, _) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, mut reserve_data) = testutils::default_reserve_meta();
        reserve_data.d_rate = 1_345_678_123_000;
        reserve_data.b_rate = 1_123_456_789_000;
        reserve_data.d_supply = 65_0000000;
        reserve_data.b_supply = 99_0000000;
        testutils::create_reserve(&e, &pool, &underlying, &reserve_config, &reserve_data);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool, || {
            storage::set_pool_config(&e, &pool_config);
            let reserve = Reserve::load(&e, &pool_config, &underlying);

            // (accrual: 1_002_957_375_248, util: .7864353)
            assert_eq!(reserve.data.d_rate, 1_349_657_798_173);
            assert_eq!(reserve.data.b_rate, 1_126_069_707_070);
            assert_eq!(reserve.data.ir_mod, 1_0449815);
            assert_eq!(reserve.data.d_supply, 65_0000000);
            assert_eq!(reserve.data.b_supply, 99_0000000);
            assert_eq!(reserve.data.backstop_credit, 0);
            assert_eq!(reserve.data.last_time, 617280);
        });
    }

    #[test]
    fn test_store() {
        let e = Env::default();
        e.mock_all_auths();

        e.ledger().set(LedgerInfo {
            timestamp: 123456 * 5,
            protocol_version: 22,
            sequence_number: 123456,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let oracle = Address::generate(&e);

        let (underlying, _) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, mut reserve_data) = testutils::default_reserve_meta();
        reserve_data.d_rate = 1_345_678_123_000;
        reserve_data.b_rate = 1_123_456_789_000;
        reserve_data.d_supply = 65_0000000;
        reserve_data.b_supply = 99_0000000;
        testutils::create_reserve(&e, &pool, &underlying, &reserve_config, &reserve_data);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_2000000,
            status: 0,
            max_positions: 5,
        };
        e.as_contract(&pool, || {
            storage::set_pool_config(&e, &pool_config);
            let reserve = Reserve::load(&e, &pool_config, &underlying);
            reserve.store(&e);

            let reserve_data = storage::get_res_data(&e, &underlying);

            // (accrual: 1_002_957_375_248, util: .7864353)
            assert_eq!(reserve_data.d_rate, 1_349_657_798_173);
            assert_eq!(reserve_data.b_rate, 1_125_547_124_242);
            assert_eq!(reserve_data.ir_mod, 1_0449815);
            assert_eq!(reserve_data.d_supply, 65_0000000);
            assert_eq!(reserve_data.b_supply, 99_0000000);
            assert_eq!(reserve_data.backstop_credit, 0_0517357);
            assert_eq!(reserve_data.last_time, 617280);
        });
    }

    #[test]
    fn test_utilization() {
        let e = Env::default();

        let mut reserve = testutils::default_reserve(&e);
        reserve.data.d_rate = 1_345_678_123_000;
        reserve.data.b_rate = 1_123_456_789_000;
        reserve.data.b_supply = 99_0000000;
        reserve.data.d_supply = 65_0000000;

        let result = reserve.utilization(&e);

        assert_eq!(result, 0_7864353);
    }

    #[test]
    fn test_require_utilization_below_max_pass() {
        let e = Env::default();

        let mut reserve = testutils::default_reserve(&e);
        reserve.data.b_supply = 99_0000000;
        reserve.data.d_supply = 65_0000000;

        reserve.require_utilization_below_max(&e);
        // no panic
        assert!(true);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1207)")]
    fn test_require_utilization_under_max_panic() {
        let e = Env::default();

        let mut reserve = testutils::default_reserve(&e);
        reserve.data.b_supply = 100_0000000;
        reserve.data.d_supply = 95_0000100;

        reserve.require_utilization_below_max(&e);
    }

    /***** Token Transfer Math *****/

    #[test]
    fn test_to_asset_from_d_token() {
        let e = Env::default();

        let mut reserve = testutils::default_reserve(&e);
        reserve.data.d_rate = 1_321_834_961_000;
        reserve.data.b_supply = 99_0000000;
        reserve.data.d_supply = 65_0000000;

        let result = reserve.to_asset_from_d_token(&e, 1_1234567);

        assert_eq!(result, 1_4850244);
    }

    #[test]
    fn test_to_asset_from_b_token() {
        let e = Env::default();

        let mut reserve = testutils::default_reserve(&e);
        reserve.data.b_rate = 1_321_834_961_000;
        reserve.data.b_supply = 99_0000000;
        reserve.data.d_supply = 65_0000000;

        let result = reserve.to_asset_from_b_token(&e, 1_1234567);

        assert_eq!(result, 1_4850243);
    }

    #[test]
    fn test_to_effective_asset_from_d_token() {
        let e = Env::default();

        let mut reserve = testutils::default_reserve(&e);
        reserve.data.d_rate = 1_321_834_961_000;
        reserve.data.b_supply = 99_0000000;
        reserve.data.d_supply = 65_0000000;
        reserve.config.l_factor = 1_1000000;

        let result = reserve.to_effective_asset_from_d_token(&e, 1_1234567);

        assert_eq!(result, 1_3500222);
    }

    #[test]
    fn test_to_effective_asset_from_b_token() {
        let e = Env::default();

        let mut reserve = testutils::default_reserve(&e);
        reserve.data.b_rate = 1_321_834_961_000;
        reserve.data.b_supply = 99_0000000;
        reserve.data.d_supply = 65_0000000;
        reserve.config.c_factor = 0_8500000;

        let result = reserve.to_effective_asset_from_b_token(&e, 1_1234567);

        assert_eq!(result, 1_2622706);
    }

    #[test]
    fn test_total_liabilities() {
        let e = Env::default();

        let mut reserve = testutils::default_reserve(&e);
        reserve.data.d_rate = 1_823_912_692_000;
        reserve.data.b_supply = 99_0000000;
        reserve.data.d_supply = 65_0000000;

        let result = reserve.total_liabilities(&e);

        assert_eq!(result, 118_5543250);
    }

    #[test]
    fn test_total_supply() {
        let e = Env::default();

        let mut reserve = testutils::default_reserve(&e);
        reserve.data.b_rate = 1_823_912_692_000;
        reserve.data.b_supply = 99_0000000;
        reserve.data.d_supply = 65_0000000;

        let result = reserve.total_supply(&e);

        assert_eq!(result, 180_5673565);
    }

    #[test]
    fn test_to_d_token_up() {
        let e = Env::default();

        let mut reserve = testutils::default_reserve(&e);
        reserve.data.d_rate = 1_321_834_961_999;
        reserve.data.b_supply = 99_0000000;
        reserve.data.d_supply = 65_0000000;

        let result = reserve.to_d_token_up(&e, 1_4850243);

        assert_eq!(result, 1_1234567);
    }

    #[test]
    fn test_to_d_token_down() {
        let e = Env::default();

        let mut reserve = testutils::default_reserve(&e);
        reserve.data.d_rate = 1_321_834_961_000;
        reserve.data.b_supply = 99_0000000;
        reserve.data.d_supply = 65_0000000;

        let result = reserve.to_d_token_down(&e, 1_4850243);

        assert_eq!(result, 1_1234566);
    }

    #[test]
    fn test_to_b_token_up() {
        let e = Env::default();

        let mut reserve = testutils::default_reserve(&e);
        reserve.data.b_rate = 1_321_834_961_999;
        reserve.data.b_supply = 99_0000000;
        reserve.data.d_supply = 65_0000000;

        let result = reserve.to_b_token_up(&e, 1_4850243);

        assert_eq!(result, 1_1234567);
    }

    #[test]
    fn test_to_b_token_down() {
        let e = Env::default();

        let mut reserve = testutils::default_reserve(&e);
        reserve.data.b_rate = 1_321_834_961_000;
        reserve.data.b_supply = 99_0000000;
        reserve.data.d_supply = 65_0000000;

        let result = reserve.to_b_token_down(&e, 1_4850243);

        assert_eq!(result, 1_1234566);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1223)")]
    fn test_require_action_allowed_panics_if_supply_disabled_asset() {
        let e = Env::default();

        let mut reserve = testutils::default_reserve(&e);
        reserve.config.enabled = false;

        reserve.require_action_allowed(&e, RequestType::SupplyCollateral as u32);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1223)")]
    fn test_require_action_allowed_panics_if_borrow_disabled_asset() {
        let e = Env::default();

        let mut reserve = testutils::default_reserve(&e);
        reserve.config.enabled = false;

        reserve.require_action_allowed(&e, RequestType::Borrow as u32);
    }

    #[test]
    fn test_require_action_allowed_passed_if_withdraw_or_repay() {
        let e = Env::default();

        let mut reserve = testutils::default_reserve(&e);
        reserve.config.enabled = false;

        reserve.require_action_allowed(&e, RequestType::Withdraw as u32);
        reserve.require_action_allowed(&e, RequestType::WithdrawCollateral as u32);
        reserve.require_action_allowed(&e, RequestType::Repay as u32);
    }

    #[test]
    fn test_accrue() {
        let e = Env::default();
        e.mock_all_auths();

        e.ledger().set(LedgerInfo {
            timestamp: 123456 * 5,
            protocol_version: 22,
            sequence_number: 123456,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let mut reserve = testutils::default_reserve(&e);
        reserve.data.backstop_credit = 0_1234567;

        reserve.accrue(&e, 0_2000000, 100_0000000);
        assert_eq!(reserve.data.backstop_credit, 20_0000000 + 0_1234567);
        assert_eq!(reserve.data.b_rate, 1_800_000_000_000);
        assert_eq!(reserve.data.last_time, 0);
    }

    #[test]
    fn test_accrue_negative_delta_no_change() {
        let e = Env::default();
        e.mock_all_auths();

        e.ledger().set(LedgerInfo {
            timestamp: 123456 * 5,
            protocol_version: 22,
            sequence_number: 123456,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let mut reserve = testutils::default_reserve(&e);
        reserve.data.backstop_credit = 0_1234567;

        reserve.accrue(&e, 0_2000000, -10_0000000);
        assert_eq!(reserve.data.backstop_credit, 0_1234567);
        assert_eq!(reserve.data.b_rate, 1_000_000_000_000);
        assert_eq!(reserve.data.last_time, 0);
    }
}
