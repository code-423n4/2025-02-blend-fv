use cast::i128;
use soroban_fixed_point_math::SorobanFixedPoint;
use soroban_sdk::{map, panic_with_error, Address, Env, Vec};

use crate::auctions::auction::AuctionData;
use crate::pool::{Pool, PositionData, User};
use crate::Positions;
use crate::{errors::PoolError, storage};

use super::AuctionType;

pub fn create_user_liq_auction_data(
    e: &Env,
    user: &Address,
    bid: &Vec<Address>,
    lot: &Vec<Address>,
    percent: u32,
) -> AuctionData {
    if user == &e.current_contract_address() || user == &storage::get_backstop(e) {
        panic_with_error!(e, PoolError::InvalidLiquidation);
    }
    if storage::has_auction(e, &(AuctionType::UserLiquidation as u32), user) {
        panic_with_error!(e, PoolError::AuctionInProgress);
    }
    if percent > 100 || percent == 0 {
        panic_with_error!(e, PoolError::InvalidLiquidation);
    }

    let mut liquidation_quote = AuctionData {
        bid: map![e],
        lot: map![e],
        block: e.ledger().sequence() + 1,
    };
    let mut full_liquidation_quote = AuctionData {
        bid: map![e],
        lot: map![e],
        block: e.ledger().sequence() + 1,
    };
    let mut pool = Pool::load(e);
    if pool.config.max_positions < (lot.len() + bid.len()) {
        panic_with_error!(e, PoolError::MaxPositionsExceeded);
    }

    // this is used for checking the liquidation percent and should NOT be set
    let mut user_state = User::load(e, user);
    let reserve_list = storage::get_res_list(e);
    let position_data = PositionData::calculate_from_positions(e, &mut pool, &user_state.positions);

    // ensure the user has less collateral than liabilities
    if position_data.liability_base < position_data.collateral_base {
        panic_with_error!(e, PoolError::InvalidLiquidation);
    }

    // build position data from included assets
    let mut positions_auctioned = Positions::env_default(e);
    for bid_asset in bid {
        // these will be cached if the bid is valid
        let reserve = pool.load_reserve(e, &bid_asset, false);
        match user_state.positions.liabilities.get(reserve.config.index) {
            Some(amount) => {
                positions_auctioned
                    .liabilities
                    .set(reserve.config.index, amount);
            }
            None => {
                panic_with_error!(e, PoolError::InvalidBid);
            }
        }
    }
    if positions_auctioned.liabilities.len() == 0 {
        panic_with_error!(e, PoolError::InvalidBid);
    }
    for lot_asset in lot {
        // these will be cached if the lot is valid
        let reserve = pool.load_reserve(e, &lot_asset, false);
        match user_state.positions.collateral.get(reserve.config.index) {
            Some(amount) => {
                positions_auctioned
                    .collateral
                    .set(reserve.config.index, amount);
            }
            None => {
                panic_with_error!(e, PoolError::InvalidLot);
            }
        }
    }
    if positions_auctioned.collateral.len() == 0 {
        panic_with_error!(e, PoolError::InvalidLot);
    }
    let position_data_inc =
        PositionData::calculate_from_positions(e, &mut pool, &positions_auctioned);
    let is_all_collateral = position_data_inc.collateral_raw == position_data.collateral_raw;
    let is_all_positions =
        is_all_collateral && position_data_inc.liability_raw == position_data.liability_raw;

    // a full liquidation is when all positions are liquidated and the liquidation percent is >95
    let is_full_liquidation = is_all_positions && percent > 95;

    // Full liquidations default to 100% liquidations.
    // To safely check this, calculate the liquidation at 95%, and verify the liquidation
    // is too small.
    let percent_liquidated_to_check = if is_full_liquidation { 95u32 } else { percent };

    let percent_liquidated_i128_scaled =
        i128(percent_liquidated_to_check) * position_data.scalar / 100; // scale to decimal form with scalar decimals

    // ensure liquidation size is fair and the collateral is large enough to allow for the auction to price the liquidation
    let avg_cf = position_data_inc.collateral_base.fixed_div_floor(
        e,
        &position_data_inc.collateral_raw,
        &position_data_inc.scalar,
    );
    // avg_lf is the inverse of the average liability factor
    let avg_lf = position_data_inc.liability_base.fixed_div_floor(
        e,
        &position_data_inc.liability_raw,
        &position_data_inc.scalar,
    );
    let est_incentive = (position_data_inc.scalar
        - avg_cf.fixed_div_ceil(e, &avg_lf, &position_data_inc.scalar))
    .fixed_div_ceil(
        e,
        &(2 * position_data_inc.scalar),
        &position_data_inc.scalar,
    ) + position_data_inc.scalar;

    let est_withdrawn_collateral = position_data_inc
        .liability_raw
        .fixed_mul_floor(
            e,
            &percent_liquidated_i128_scaled,
            &position_data_inc.scalar,
        )
        .fixed_mul_floor(e, &est_incentive, &position_data_inc.scalar);
    let mut est_withdrawn_collateral_pct = est_withdrawn_collateral.fixed_div_ceil(
        e,
        &position_data_inc.collateral_raw,
        &position_data_inc.scalar,
    );

    // estimated lot exceedes the collateral available in the included positions
    if est_withdrawn_collateral_pct > position_data_inc.scalar {
        est_withdrawn_collateral_pct = position_data_inc.scalar;
        // if the included collateral is not all of the users collateral, panic,
        // as the missing collateral should be included in the liquidation to avoid
        // potentially bad liquidations
        if !is_all_collateral {
            panic_with_error!(e, PoolError::InvalidLiquidation);
        }
    }

    for (asset, amount) in positions_auctioned.collateral.iter() {
        let res_asset_address = reserve_list.get_unchecked(asset);
        let b_tokens_removed =
            amount.fixed_mul_ceil(e, &est_withdrawn_collateral_pct, &position_data.scalar);
        liquidation_quote
            .lot
            .set(res_asset_address.clone(), b_tokens_removed);
        full_liquidation_quote.lot.set(res_asset_address, amount);
    }

    for (asset, amount) in positions_auctioned.liabilities.iter() {
        let res_asset_address = reserve_list.get_unchecked(asset);
        let d_tokens_removed =
            amount.fixed_mul_ceil(e, &percent_liquidated_i128_scaled, &position_data.scalar);
        liquidation_quote
            .bid
            .set(res_asset_address.clone(), d_tokens_removed);
        full_liquidation_quote.bid.set(res_asset_address, amount);
    }

    user_state.rm_positions(
        e,
        &mut pool,
        liquidation_quote.lot.clone(),
        liquidation_quote.bid.clone(),
    );
    let new_data = PositionData::calculate_from_positions(e, &mut pool, &user_state.positions);

    if is_full_liquidation {
        // A full user liquidation was requested, validate that a full liquidation is not too large.
        // If the user has enough collateral to create the liquidation auction, validate that the
        // 95% liquidation is not too large. That is, if a user can be liquidated to 95%, they can
        // be liquidated fully. This helps prevent edge cases due to liquidation percentages
        // being harder to calculate between as it approaches 100.
        if est_withdrawn_collateral < position_data.collateral_raw
            && new_data.is_hf_over(e, 1_1500000)
        {
            panic_with_error!(e, PoolError::InvalidLiqTooLarge)
        };
        full_liquidation_quote
    } else {
        // Post-liq health factor must be under 1.15
        if new_data.is_hf_over(e, 1_1500000) {
            panic_with_error!(e, PoolError::InvalidLiqTooLarge)
        };

        // Post-liq heath factor must be over 1.03
        if new_data.is_hf_under(e, 1_0300000) {
            panic_with_error!(e, PoolError::InvalidLiqTooSmall)
        };
        liquidation_quote
    }
}

pub fn fill_user_liq_auction(
    e: &Env,
    pool: &mut Pool,
    auction_data: &AuctionData,
    user: &Address,
    filler_state: &mut User,
) {
    let mut user_state = User::load(e, user);
    user_state.rm_positions(e, pool, auction_data.lot.clone(), auction_data.bid.clone());
    filler_state.add_positions(e, pool, auction_data.lot.clone(), auction_data.bid.clone());
    user_state.store(e);
}

#[cfg(test)]
mod tests {

    use crate::{
        auctions::auction::AuctionType,
        pool::Positions,
        storage::{self, PoolConfig},
        testutils::{self, create_pool},
    };

    use super::*;
    use sep_40_oracle::testutils::Asset;
    use soroban_sdk::{
        testutils::{Address as AddressTestTrait, Ledger, LedgerInfo},
        vec, Symbol,
    };

    #[test]
    #[should_panic(expected = "Error(Contract, #1212)")]
    fn test_create_liquidation_already_in_progress() {
        let e = Env::default();
        e.mock_all_auths();

        let pool_address = create_pool(&e);
        let (oracle, _) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        let samwise = Address::generate(&e);

        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 100,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let liq_pct = 50;

        let auction_data = AuctionData {
            bid: map![&e],
            lot: map![&e],
            block: 50,
        };
        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);
            storage::set_auction(
                &e,
                &(AuctionType::UserLiquidation as u32),
                &samwise,
                &auction_data,
            );
            create_user_liq_auction_data(&e, &samwise, &vec![&e], &vec![&e], liq_pct);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1211)")]
    fn test_create_liquidation_user_is_pool() {
        let e = Env::default();
        e.mock_all_auths();

        let pool_address = create_pool(&e);
        let (oracle, _) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 100,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let liq_pct = 50;
        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);
            create_user_liq_auction_data(&e, &pool_address, &vec![&e], &vec![&e], liq_pct);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1211)")]
    fn test_create_liquidation_user_is_backstop() {
        let e = Env::default();
        e.mock_all_auths();

        let pool_address = create_pool(&e);
        let (oracle, _) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 100,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let liq_pct = 50;
        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);
            create_user_liq_auction_data(&e, &backstop_address, &vec![&e], &vec![&e], liq_pct);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1211)")]
    fn test_create_liquidation_percent_zero() {
        let e = Env::default();
        e.mock_all_auths();

        let pool_address = create_pool(&e);
        let (oracle, _) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 100,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let liq_pct = 0;
        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);
            create_user_liq_auction_data(&e, &backstop_address, &vec![&e], &vec![&e], liq_pct);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1211)")]
    fn test_create_liquidation_percent_over_100() {
        let e = Env::default();
        e.mock_all_auths();

        let pool_address = create_pool(&e);
        let (oracle, _) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 100,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let liq_pct = 101;
        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);
            create_user_liq_auction_data(&e, &backstop_address, &vec![&e], &vec![&e], liq_pct);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1221)")]
    fn test_create_user_liquidation_invalid_bid_empty() {
        let e = Env::default();

        e.mock_all_auths();
        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 50,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);

        let pool_address = create_pool(&e);
        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        // creating reserves for a pool exhausts the budget
        e.cost_estimate().budget().reset_unlimited();
        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_100_000_000_000;
        reserve_data_0.d_rate = 1_150_000_000_000;
        reserve_config_0.c_factor = 0_8500000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_data_1.last_time = 12345;
        reserve_data_1.b_rate = 1_200_000_000_000;
        reserve_data_1.d_rate = 1_300_000_000_000;
        reserve_config_1.c_factor = 0_8000000;
        reserve_config_1.l_factor = 0_7500000;
        reserve_config_1.index = 1;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
            ],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 2_0000000, 4_0000000]);

        let liq_pct = 50;
        let positions: Positions = Positions {
            collateral: map![&e, (reserve_config_0.index, 100_0000000),],
            liabilities: map![&e, (reserve_config_1.index, 30_0000000),],
            supply: map![&e],
        };
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);

            create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e],
                &vec![&e, underlying_0.clone()],
                liq_pct,
            );
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1221)")]
    fn test_create_user_liquidation_invalid_bid_no_position() {
        let e = Env::default();

        e.mock_all_auths();
        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 50,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);

        let pool_address = create_pool(&e);
        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        // creating reserves for a pool exhausts the budget
        e.cost_estimate().budget().reset_unlimited();
        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_100_000_000_000;
        reserve_data_0.d_rate = 1_150_000_000_000;
        reserve_config_0.c_factor = 0_8500000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_data_1.last_time = 12345;
        reserve_data_1.b_rate = 1_200_000_000_000;
        reserve_data_1.d_rate = 1_300_000_000_000;
        reserve_config_1.c_factor = 0_8000000;
        reserve_config_1.l_factor = 0_7500000;
        reserve_config_1.index = 1;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
            ],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 2_0000000, 4_0000000]);

        let liq_pct = 50;
        let positions: Positions = Positions {
            collateral: map![&e, (reserve_config_0.index, 100_0000000),],
            liabilities: map![&e, (reserve_config_1.index, 30_0000000),],
            supply: map![&e],
        };
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);

            create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e, underlying_0.clone()],
                &vec![&e, underlying_0.clone()],
                liq_pct,
            );
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1222)")]
    fn test_create_user_liquidation_invalid_lot_empty() {
        let e = Env::default();

        e.mock_all_auths();
        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 50,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);

        let pool_address = create_pool(&e);
        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        // creating reserves for a pool exhausts the budget
        e.cost_estimate().budget().reset_unlimited();
        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_100_000_000_000;
        reserve_data_0.d_rate = 1_150_000_000_000;
        reserve_config_0.c_factor = 0_8500000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_data_1.last_time = 12345;
        reserve_data_1.b_rate = 1_200_000_000_000;
        reserve_data_1.d_rate = 1_300_000_000_000;
        reserve_config_1.c_factor = 0_8000000;
        reserve_config_1.l_factor = 0_7500000;
        reserve_config_1.index = 1;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
            ],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 2_0000000, 4_0000000]);

        let liq_pct = 50;
        let positions: Positions = Positions {
            collateral: map![&e, (reserve_config_0.index, 100_0000000),],
            liabilities: map![&e, (reserve_config_1.index, 30_0000000),],
            supply: map![&e],
        };
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);

            create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e, underlying_1.clone()],
                &vec![&e],
                liq_pct,
            );
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1222)")]
    fn test_create_user_liquidation_invalid_lot_no_position() {
        let e = Env::default();

        e.mock_all_auths();
        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 50,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);

        let pool_address = create_pool(&e);
        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        // creating reserves for a pool exhausts the budget
        e.cost_estimate().budget().reset_unlimited();
        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_100_000_000_000;
        reserve_data_0.d_rate = 1_150_000_000_000;
        reserve_config_0.c_factor = 0_8500000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_data_1.last_time = 12345;
        reserve_data_1.b_rate = 1_200_000_000_000;
        reserve_data_1.d_rate = 1_300_000_000_000;
        reserve_config_1.c_factor = 0_8000000;
        reserve_config_1.l_factor = 0_7500000;
        reserve_config_1.index = 1;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
            ],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 2_0000000, 4_0000000]);

        let liq_pct = 50;
        let positions: Positions = Positions {
            collateral: map![&e, (reserve_config_0.index, 100_0000000),],
            liabilities: map![&e, (reserve_config_1.index, 30_0000000),],
            supply: map![&e],
        };
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);

            create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e, underlying_1.clone()],
                &vec![&e, underlying_1.clone()],
                liq_pct,
            );
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1208)")]
    fn test_create_user_liquidation_checks_max_positions() {
        let e = Env::default();

        e.mock_all_auths();
        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 50,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);

        let pool_address = create_pool(&e);
        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        // creating reserves for a pool exhausts the budget
        e.cost_estimate().budget().reset_unlimited();
        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_100_000_000_000;
        reserve_config_0.c_factor = 0_8500000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_data_1.b_rate = 1_200_000_000_000;
        reserve_config_1.c_factor = 0_7500000;
        reserve_config_1.l_factor = 0_7500000;
        reserve_data_1.last_time = 12345;
        reserve_config_1.index = 1;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        let (underlying_2, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_2, reserve_data_2) = testutils::default_reserve_meta();
        reserve_config_2.c_factor = 0_0000000;
        reserve_config_2.l_factor = 0_7000000;
        reserve_config_2.index = 2;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_2,
            &reserve_config_2,
            &reserve_data_2,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
                Asset::Stellar(underlying_2.clone()),
            ],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 2_0000000, 4_0000000, 50_0000000]);

        let liq_pct = 45;
        let positions: Positions = Positions {
            collateral: map![
                &e,
                (reserve_config_0.index, 90_9100000),
                (reserve_config_1.index, 04_5800000),
            ],
            liabilities: map![&e, (reserve_config_2.index, 02_7500000),],
            supply: map![&e],
        };
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 2,
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);

            create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e, underlying_2.clone()],
                &vec![&e, underlying_0.clone(), underlying_1.clone()],
                liq_pct,
            );
        });
    }

    #[test]
    fn test_create_user_liquidation_auction_normal_scalars() {
        let e = Env::default();

        e.mock_all_auths();
        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 50,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);

        let pool_address = create_pool(&e);
        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        // creating reserves for a pool exhausts the budget
        e.cost_estimate().budget().reset_unlimited();
        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_100_000_000_000;
        reserve_config_0.c_factor = 0_8500000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_data_1.b_rate = 1_200_000_000_000;
        reserve_config_1.c_factor = 0_7500000;
        reserve_config_1.l_factor = 0_7500000;
        reserve_data_1.last_time = 12345;
        reserve_config_1.index = 1;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        let (underlying_2, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_2, reserve_data_2) = testutils::default_reserve_meta();
        reserve_config_2.c_factor = 0_0000000;
        reserve_config_2.l_factor = 0_7000000;
        reserve_config_2.index = 2;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_2,
            &reserve_config_2,
            &reserve_data_2,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
                Asset::Stellar(underlying_2.clone()),
            ],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 2_0000000, 4_0000000, 50_0000000]);

        let liq_pct = 45;
        let positions: Positions = Positions {
            collateral: map![
                &e,
                (reserve_config_0.index, 90_9100000),
                (reserve_config_1.index, 04_5800000),
            ],
            liabilities: map![&e, (reserve_config_2.index, 02_7500000),],
            supply: map![&e],
        };
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);

            let result = create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e, underlying_2.clone()],
                &vec![&e, underlying_0.clone(), underlying_1.clone()],
                liq_pct,
            );
            assert_eq!(result.block, 51);
            assert_eq!(result.bid.get_unchecked(underlying_2), 1_2375000);
            assert_eq!(result.bid.len(), 1);
            assert_eq!(result.lot.get_unchecked(underlying_0), 30_5595329);
            assert_eq!(result.lot.get_unchecked(underlying_1), 1_5395739);
            assert_eq!(result.lot.len(), 2);
        });
    }

    #[test]
    fn test_create_user_liquidation_auction_weird_scalar() {
        let e = Env::default();

        e.mock_all_auths();
        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 50,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);

        let pool_address = create_pool(&e);
        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        // creating reserves for a pool exhausts the budget
        e.cost_estimate().budget().reset_unlimited();
        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_000_206_159_000;
        reserve_config_0.c_factor = 0_9000000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_config_1.c_factor = 0_0000000;
        reserve_config_1.l_factor = 0_9000000;
        reserve_config_1.index = 1;
        reserve_data_1.d_rate = 1_000_201_748_000;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
            ],
            &14,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 1418501_2444444, 1_0261166_9700969]);

        let liq_pct = 69;
        let positions: Positions = Positions {
            collateral: map![&e, (reserve_config_0.index, 8999_1357639),],
            liabilities: map![&e, (reserve_config_1.index, 1059_5526742),],
            supply: map![&e],
        };
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);

            let result = create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e, underlying_1.clone()],
                &vec![&e, underlying_0.clone()],
                liq_pct,
            );
            assert_eq!(result.block, 51);
            assert_eq!(result.bid.get_unchecked(underlying_1), 731_0913452);
            assert_eq!(result.bid.len(), 1);
            assert_eq!(result.lot.get_unchecked(underlying_0), 5791_1010712);
            assert_eq!(result.lot.len(), 1);
        });
    }

    #[test]
    fn test_create_user_liquidation_auction_full_liquidation() {
        let e = Env::default();

        e.mock_all_auths();
        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 50,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);

        let pool_address = create_pool(&e);
        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        // creating reserves for a pool exhausts the budget
        e.cost_estimate().budget().reset_unlimited();
        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_000_206_159_000;
        reserve_config_0.c_factor = 0_9000000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_config_1.c_factor = 0_0000000;
        reserve_config_1.l_factor = 0_9000000;
        reserve_config_1.index = 1;
        reserve_config_1.decimals = 6;
        reserve_data_1.d_rate = 1_000_201_748_000;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
            ],
            &5,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 1_00000, 1_00000]);

        let liq_pct = 100;
        let positions: Positions = Positions {
            collateral: map![&e, (reserve_config_0.index, 8_000_0000),],
            liabilities: map![&e, (reserve_config_1.index, 100_000_000),],
            supply: map![&e],
        };
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);

            let result = create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e, underlying_1.clone()],
                &vec![&e, underlying_0.clone()],
                liq_pct,
            );
            assert_eq!(result.block, 51);
            assert_eq!(result.bid.get_unchecked(underlying_1), 10_0000000);
            assert_eq!(result.bid.len(), 1);
            assert_eq!(result.lot.get_unchecked(underlying_0), 8_0000000);
            assert_eq!(result.lot.len(), 1);
        });
    }

    #[test]
    fn test_create_user_liquidation_auction_over_95_percent_liqs_fully() {
        let e = Env::default();
        e.mock_all_auths();
        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 50,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });
        e.cost_estimate().budget().reset_unlimited();

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);

        let pool_address = create_pool(&e);
        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_000_206_159_000;
        reserve_config_0.c_factor = 0_9000000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_config_1.c_factor = 0_5000000;
        reserve_config_1.l_factor = 0_8000000;
        reserve_config_1.index = 1;
        reserve_data_1.d_rate = 1_050_001_748_000;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
            ],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 1_0000000]);

        let liq_pct = 96;
        // true liquidation percent between 99-100%
        let positions: Positions = Positions {
            collateral: map![
                &e,
                (reserve_config_1.index, 75_500_0000),
                (reserve_config_0.index, 50_000_0000)
            ],
            liabilities: map![
                &e,
                (reserve_config_1.index, 50_000_0000),
                (reserve_config_0.index, 50_000_0000)
            ],
            supply: map![&e],
        };
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);

            let result = create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e, underlying_0.clone(), underlying_1.clone()],
                &vec![&e, underlying_0.clone(), underlying_1.clone()],
                liq_pct,
            );
            assert_eq!(result.block, 51);
            assert_eq!(result.bid.get_unchecked(underlying_1.clone()), 50_000_0000);
            assert_eq!(result.bid.get_unchecked(underlying_0.clone()), 50_000_0000);
            assert_eq!(result.bid.len(), 2);
            assert_eq!(result.lot.get_unchecked(underlying_1.clone()), 75_500_0000);
            assert_eq!(result.lot.get_unchecked(underlying_0.clone()), 50_000_0000);
            assert_eq!(result.lot.len(), 2);
        });
    }

    #[test]
    fn test_create_user_liquidation_auction_95_safe_can_liq_fully() {
        let e = Env::default();
        e.mock_all_auths();
        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 50,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });
        e.cost_estimate().budget().reset_unlimited();

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);

        let pool_address = create_pool(&e);
        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_100_000_000_000;
        reserve_data_0.d_rate = 1_150_000_000_000;
        reserve_config_0.c_factor = 0_8500000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_data_1.last_time = 12345;
        reserve_data_1.b_rate = 1_200_000_000_000;
        reserve_data_1.d_rate = 1_300_000_000_000;
        reserve_config_1.c_factor = 0_8000000;
        reserve_config_1.l_factor = 0_7500000;
        reserve_config_1.index = 1;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
            ],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 2_0000000, 4_0000000]);

        // 95% liquidation results in a hf of 1.147
        let positions: Positions = Positions {
            collateral: map![
                &e,
                (reserve_config_0.index, 100_0000000),
                (reserve_config_1.index, 100_0000000)
            ],
            liabilities: map![
                &e,
                (reserve_config_0.index, 82_7500000),
                (reserve_config_1.index, 75_0000000)
            ],
            supply: map![&e],
        };
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);

            // validate 95% liquidation is valid
            let result_95 = create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e, underlying_0.clone(), underlying_1.clone()],
                &vec![&e, underlying_0.clone(), underlying_1.clone()],
                95,
            );
            assert_eq!(result_95.block, 51);
            assert_eq!(
                result_95.bid.get_unchecked(underlying_0.clone()),
                78_6125000
            );
            assert_eq!(
                result_95.bid.get_unchecked(underlying_1.clone()),
                71_2500000
            );
            assert_eq!(result_95.bid.len(), 2);
            assert_eq!(
                result_95.lot.get_unchecked(underlying_0.clone()),
                92_6529600
            );
            assert_eq!(
                result_95.lot.get_unchecked(underlying_1.clone()),
                92_6529600
            );
            assert_eq!(result_95.lot.len(), 2);

            // validate if 95% is valid, a full liquidation can be completed
            let result_100 = create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e, underlying_0.clone(), underlying_1.clone()],
                &vec![&e, underlying_0.clone(), underlying_1.clone()],
                100,
            );
            assert_eq!(result_100.block, 51);
            assert_eq!(
                result_100.bid.get_unchecked(underlying_0.clone()),
                82_7500000
            );
            assert_eq!(
                result_100.bid.get_unchecked(underlying_1.clone()),
                75_0000000
            );
            assert_eq!(result_100.bid.len(), 2);
            assert_eq!(
                result_100.lot.get_unchecked(underlying_0.clone()),
                100_0000000
            );
            assert_eq!(
                result_100.lot.get_unchecked(underlying_1.clone()),
                100_0000000
            );
            assert_eq!(result_100.lot.len(), 2);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1213)")]
    fn test_create_user_liquidation_auction_bad_full_liq() {
        let e = Env::default();

        e.mock_all_auths();
        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 50,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);

        let pool_address = create_pool(&e);

        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        // creating reserves for a pool exhausts the budget
        e.cost_estimate().budget().reset_unlimited();
        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_100_000_000_000;
        reserve_config_0.c_factor = 0_8500000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_data_1.b_rate = 1_200_000_000_000;
        reserve_config_1.c_factor = 0_7500000;
        reserve_config_1.l_factor = 0_7500000;
        reserve_data_1.last_time = 12345;
        reserve_config_1.index = 1;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        let (underlying_2, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_2, reserve_data_2) = testutils::default_reserve_meta();
        reserve_config_2.c_factor = 0_0000000;
        reserve_config_2.l_factor = 0_7000000;
        reserve_config_2.index = 2;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_2,
            &reserve_config_2,
            &reserve_data_2,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
                Asset::Stellar(underlying_2.clone()),
            ],
            &8,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 2_0000000_0, 4_0000000_0, 50_0000000_0]);

        let liq_pct = 100;
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        let positions: Positions = Positions {
            collateral: map![
                &e,
                (reserve_config_0.index, 90_9100000),
                (reserve_config_1.index, 04_5800000),
            ],
            liabilities: map![&e, (reserve_config_2.index, 02_7500000),],
            supply: map![&e],
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);

            create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e, underlying_2.clone()],
                &vec![&e, underlying_0.clone(), underlying_1.clone()],
                liq_pct,
            );
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1213)")]
    fn test_create_user_liquidation_auction_too_large() {
        let e = Env::default();

        e.mock_all_auths();
        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 50,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);

        let pool_address = create_pool(&e);

        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        // creating reserves for a pool exhausts the budget
        e.cost_estimate().budget().reset_unlimited();
        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_100_000_000_000;
        reserve_config_0.c_factor = 0_8500000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_data_1.b_rate = 1_200_000_000_000;
        reserve_config_1.c_factor = 0_7500000;
        reserve_config_1.l_factor = 0_7500000;
        reserve_data_1.last_time = 12345;
        reserve_config_1.index = 1;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        let (underlying_2, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_2, reserve_data_2) = testutils::default_reserve_meta();
        reserve_config_2.c_factor = 0_0000000;
        reserve_config_2.l_factor = 0_7000000;
        reserve_config_2.index = 2;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_2,
            &reserve_config_2,
            &reserve_data_2,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
                Asset::Stellar(underlying_2.clone()),
            ],
            &6,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 2_000000, 4_000000, 50_000000]);

        let liq_pct = 46;
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        let positions: Positions = Positions {
            collateral: map![
                &e,
                (reserve_config_0.index, 90_9100000),
                (reserve_config_1.index, 04_5800000),
            ],
            liabilities: map![&e, (reserve_config_2.index, 02_7500000),],
            supply: map![&e],
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);

            create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e, underlying_2.clone()],
                &vec![&e, underlying_0.clone(), underlying_1.clone()],
                liq_pct,
            );
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1214)")]
    fn test_create_user_liquidation_auction_too_small() {
        let e = Env::default();

        e.mock_all_auths();
        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 50,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);

        let pool_address = create_pool(&e);

        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        // creating reserves for a pool exhausts the budget
        e.cost_estimate().budget().reset_unlimited();
        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_100_000_000_000;
        reserve_config_0.c_factor = 0_8500000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_data_1.b_rate = 1_200_000_000_000;
        reserve_config_1.c_factor = 0_7500000;
        reserve_config_1.l_factor = 0_7500000;
        reserve_data_1.last_time = 12345;
        reserve_config_1.index = 1;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        let (underlying_2, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_2, reserve_data_2) = testutils::default_reserve_meta();
        reserve_config_2.c_factor = 0_0000000;
        reserve_config_2.l_factor = 0_7000000;
        reserve_config_2.index = 2;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_2,
            &reserve_config_2,
            &reserve_data_2,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
                Asset::Stellar(underlying_2.clone()),
            ],
            &5,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 2_00000, 4_00000, 50_00000]);

        let liq_pct = 25;
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        let positions: Positions = Positions {
            collateral: map![
                &e,
                (reserve_config_0.index, 90_9100000),
                (reserve_config_1.index, 04_5800000),
            ],
            liabilities: map![&e, (reserve_config_2.index, 02_7500000),],
            supply: map![&e],
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);

            create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e, underlying_2.clone()],
                &vec![&e, underlying_0.clone(), underlying_1.clone()],
                liq_pct,
            );
        });
    }

    #[test]
    fn test_create_user_liquidation_partial() {
        let e = Env::default();
        e.mock_all_auths();
        e.cost_estimate().budget().reset_unlimited();

        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 50,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);

        let pool_address = create_pool(&e);
        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_100_000_000_000;
        reserve_data_0.d_rate = 1_150_000_000_000;
        reserve_config_0.c_factor = 0_8500000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_data_1.last_time = 12345;
        reserve_data_1.b_rate = 1_200_000_000_000;
        reserve_data_1.d_rate = 1_300_000_000_000;
        reserve_config_1.c_factor = 0_8000000;
        reserve_config_1.l_factor = 0_7500000;
        reserve_config_1.index = 1;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
            ],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 2_0000000, 4_0000000]);

        let liq_pct = 85;
        let positions: Positions = Positions {
            collateral: map![
                &e,
                (reserve_config_0.index, 50_0000000),
                (reserve_config_1.index, 30_0000000),
            ],
            liabilities: map![
                &e,
                (reserve_config_0.index, 30_0000000),
                (reserve_config_1.index, 20_0000000),
            ],
            supply: map![&e],
        };
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);

            let result = create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e, underlying_0.clone()],
                &vec![&e, underlying_1.clone()],
                liq_pct,
            );

            assert_eq!(result.block, 51);
            assert_eq!(result.bid.get_unchecked(underlying_0.clone()), 25_5000000);
            assert_eq!(result.bid.len(), 1);
            assert_eq!(result.lot.get_unchecked(underlying_1.clone()), 13_9293750);
            assert_eq!(result.lot.len(), 1);
        });
    }

    #[test]
    fn test_create_user_liquidation_partial_100() {
        let e = Env::default();
        e.mock_all_auths();
        e.cost_estimate().budget().reset_unlimited();

        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 50,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);

        let pool_address = create_pool(&e);
        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_100_000_000_000;
        reserve_data_0.d_rate = 1_150_000_000_000;
        reserve_config_0.c_factor = 0_8500000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_data_1.last_time = 12345;
        reserve_data_1.b_rate = 1_200_000_000_000;
        reserve_data_1.d_rate = 1_300_000_000_000;
        reserve_config_1.c_factor = 0_8000000;
        reserve_config_1.l_factor = 0_7500000;
        reserve_config_1.index = 1;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
            ],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 2_0000000, 4_0000000]);

        // liquidation can be safely filled by just liquidating a single liability
        // validate the collateral is auctioned correctly to create a fair liquidation
        // -> including the full position results in an ~60% liquidation, and a slightly larger
        //    liquidation overall due to the higher liability factor of reserve 1
        let liq_pct = 100;
        let positions: Positions = Positions {
            collateral: map![&e, (reserve_config_0.index, 100_0000000),],
            liabilities: map![
                &e,
                (reserve_config_0.index, 40_0000000),
                (reserve_config_1.index, 15_0000000),
            ],
            supply: map![&e],
        };
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);

            let result = create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e, underlying_1.clone()],
                &vec![&e, underlying_0.clone()],
                liq_pct,
            );

            assert_eq!(result.block, 51);
            assert_eq!(result.bid.get_unchecked(underlying_1.clone()), 15_0000000);
            assert_eq!(result.bid.len(), 1);
            assert_eq!(result.lot.get_unchecked(underlying_0.clone()), 41_8806900);
            assert_eq!(result.lot.len(), 1);
        });
    }

    #[test]
    fn test_create_user_liquidation_partial_0_cf_lot() {
        let e = Env::default();
        e.mock_all_auths();
        e.cost_estimate().budget().reset_unlimited();

        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 50,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);

        let pool_address = create_pool(&e);
        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_100_000_000_000;
        reserve_data_0.d_rate = 1_150_000_000_000;
        reserve_config_0.c_factor = 0_8500000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_data_1.last_time = 12345;
        reserve_data_1.b_rate = 1_200_000_000_000;
        reserve_data_1.d_rate = 1_300_000_000_000;
        reserve_config_1.c_factor = 0;
        reserve_config_1.l_factor = 0_7500000;
        reserve_config_1.index = 1;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
            ],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 2_0000000, 4_0000000]);

        // validate a liquidation of only 0 CF assets is valid
        // -> asset 1 has 0 CF
        let liq_pct = 15;
        let positions: Positions = Positions {
            collateral: map![
                &e,
                (reserve_config_0.index, 100_0000000),
                (reserve_config_1.index, 100_0000000),
            ],
            liabilities: map![&e, (reserve_config_0.index, 80_0000000),],
            supply: map![&e],
        };
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);

            let result = create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e, underlying_0.clone()],
                &vec![&e, underlying_1.clone()],
                liq_pct,
            );

            assert_eq!(result.block, 51);
            assert_eq!(result.bid.get_unchecked(underlying_0.clone()), 12_0000000);
            assert_eq!(result.bid.len(), 1);
            assert_eq!(result.lot.get_unchecked(underlying_1.clone()), 8_6250000);
            assert_eq!(result.lot.len(), 1);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1211)")]
    fn test_create_user_liquidation_partial_exclude_collateral_when_required_panics() {
        let e = Env::default();
        e.mock_all_auths();
        e.cost_estimate().budget().reset_unlimited();

        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 50,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);

        let pool_address = create_pool(&e);
        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_100_000_000_000;
        reserve_data_0.d_rate = 1_150_000_000_000;
        reserve_config_0.c_factor = 0_8500000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_data_1.last_time = 12345;
        reserve_data_1.b_rate = 1_200_000_000_000;
        reserve_data_1.d_rate = 1_300_000_000_000;
        reserve_config_1.c_factor = 0_8000000;
        reserve_config_1.l_factor = 0_7500000;
        reserve_config_1.index = 1;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
            ],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 2_0000000, 4_0000000]);

        // user holds most collateral in asset 0
        // liquidator attempts to create a bad liquidation by excluding asset 0
        // causing excess liabilities to be included in the bid
        let liq_pct = 40;
        let positions: Positions = Positions {
            collateral: map![
                &e,
                (reserve_config_0.index, 60_0000000),
                (reserve_config_1.index, 10_0000000),
            ],
            liabilities: map![&e, (reserve_config_1.index, 25_0000000),],
            supply: map![&e],
        };
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);

            create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e, underlying_1.clone()],
                &vec![&e, underlying_1.clone()],
                liq_pct,
            );
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1214)")]
    fn test_create_user_liquidation_partial_exclude_liabilities_when_required_too_small() {
        let e = Env::default();
        e.mock_all_auths();
        e.cost_estimate().budget().reset_unlimited();

        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 50,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);

        let pool_address = create_pool(&e);
        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);
        let backstop_address = Address::generate(&e);

        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_100_000_000_000;
        reserve_data_0.d_rate = 1_150_000_000_000;
        reserve_config_0.c_factor = 0_8500000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_data_1.last_time = 12345;
        reserve_data_1.b_rate = 1_200_000_000_000;
        reserve_data_1.d_rate = 1_300_000_000_000;
        reserve_config_1.c_factor = 0_8000000;
        reserve_config_1.l_factor = 0_7500000;
        reserve_config_1.index = 1;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
            ],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 2_0000000, 4_0000000]);

        // user holds most liabilities in asset 1
        // liquidator attempts to create a bad liquidation by excluding asset 1 from bid
        let liq_pct = 100;
        let positions: Positions = Positions {
            collateral: map![&e, (reserve_config_0.index, 100_0000000),],
            liabilities: map![
                &e,
                (reserve_config_0.index, 10_0000000),
                (reserve_config_1.index, 25_0000000),
            ],
            supply: map![&e],
        };
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            storage::set_backstop(&e, &backstop_address);

            create_user_liq_auction_data(
                &e,
                &samwise,
                &vec![&e, underlying_0.clone()],
                &vec![&e, underlying_0.clone()],
                liq_pct,
            );
        });
    }

    #[test]
    fn test_fill_user_liquidation_auction() {
        let e = Env::default();

        e.mock_all_auths();
        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 175,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 17280,
            min_persistent_entry_ttl: 17280,
            max_entry_ttl: 9999999,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let frodo = Address::generate(&e);

        let pool_address = create_pool(&e);

        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);

        // creating reserves for a pool exhausts the budget
        e.cost_estimate().budget().reset_unlimited();
        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_100_000_000_000;
        reserve_config_0.c_factor = 0_8500000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_data_1.b_rate = 1_200_000_000_000;
        reserve_config_1.c_factor = 0_7500000;
        reserve_config_1.l_factor = 0_7500000;
        reserve_data_1.last_time = 12345;
        reserve_config_1.index = 1;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        let (underlying_2, reserve_2_asset) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_2, reserve_data_2) = testutils::default_reserve_meta();
        reserve_config_2.c_factor = 0_0000000;
        reserve_config_2.l_factor = 0_7000000;
        reserve_config_2.index = 2;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_2,
            &reserve_config_2,
            &reserve_data_2,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
                Asset::Stellar(underlying_2.clone()),
            ],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 2_0000000, 4_0000000, 50_0000000]);

        reserve_2_asset.mint(&frodo, &0_8000000);
        reserve_2_asset.approve(&frodo, &pool_address, &i128::MAX, &1000000);

        let mut auction_data = AuctionData {
            bid: map![&e, (underlying_2.clone(), 1_2375000)],
            lot: map![
                &e,
                (underlying_0.clone(), 30_5595329),
                (underlying_1.clone(), 1_5395739)
            ],
            block: 176,
        };
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        let positions: Positions = Positions {
            collateral: map![
                &e,
                (reserve_config_0.index, 90_9100000),
                (reserve_config_1.index, 04_5800000),
            ],
            liabilities: map![&e, (reserve_config_2.index, 02_7500000),],
            supply: map![&e],
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);

            e.ledger().set(LedgerInfo {
                timestamp: 12345 + 200 * 5,
                protocol_version: 22,
                sequence_number: 176 + 200,
                network_id: Default::default(),
                base_reserve: 10,
                min_temp_entry_ttl: 17280,
                min_persistent_entry_ttl: 17280,
                max_entry_ttl: 9999999,
            });
            let mut pool = Pool::load(&e);
            let mut frodo_state = User::load(&e, &frodo);
            fill_user_liq_auction(&e, &mut pool, &mut auction_data, &samwise, &mut frodo_state);
            let frodo_positions = frodo_state.positions;
            assert_eq!(
                frodo_positions
                    .collateral
                    .get(reserve_config_0.index)
                    .unwrap(),
                30_5595329
            );
            assert_eq!(
                frodo_positions
                    .collateral
                    .get(reserve_config_1.index)
                    .unwrap(),
                1_5395739
            );
            assert_eq!(
                frodo_positions
                    .liabilities
                    .get(reserve_config_2.index)
                    .unwrap(),
                1_2375000
            );
            let samwise_positions = storage::get_user_positions(&e, &samwise);
            assert_eq!(
                samwise_positions
                    .collateral
                    .get(reserve_config_0.index)
                    .unwrap(),
                90_9100000 - 30_5595329
            );
            assert_eq!(
                samwise_positions
                    .collateral
                    .get(reserve_config_1.index)
                    .unwrap(),
                04_5800000 - 1_5395739
            );
            assert_eq!(
                samwise_positions
                    .liabilities
                    .get(reserve_config_2.index)
                    .unwrap(),
                02_7500000 - 1_2375000
            );
        });
    }

    #[test]
    fn test_fill_user_liquidation_auction_hits_target() {
        let e = Env::default();

        e.mock_all_auths();
        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 175,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 17280,
            min_persistent_entry_ttl: 17280,
            max_entry_ttl: 9999999,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let frodo = Address::generate(&e);

        let pool_address = create_pool(&e);

        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);

        // creating reserves for a pool exhausts the budget
        e.cost_estimate().budget().reset_unlimited();
        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_100_000_000_000;
        reserve_config_0.c_factor = 0_8500000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_data_1.b_rate = 1_200_000_000_000;
        reserve_config_1.c_factor = 0_7500000;
        reserve_config_1.l_factor = 0_7500000;
        reserve_data_1.last_time = 12345;
        reserve_config_1.index = 1;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        let (underlying_2, reserve_2_asset) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_2, reserve_data_2) = testutils::default_reserve_meta();
        reserve_config_2.c_factor = 0_0000000;
        reserve_config_2.l_factor = 0_7000000;
        reserve_config_2.index = 2;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_2,
            &reserve_config_2,
            &reserve_data_2,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
                Asset::Stellar(underlying_2.clone()),
            ],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 2_0000000, 4_0000000, 50_0000000]);

        reserve_2_asset.mint(&frodo, &0_8000000);
        reserve_2_asset.approve(&frodo, &pool_address, &i128::MAX, &1000000);

        let mut auction_data = AuctionData {
            bid: map![&e, (underlying_2.clone(), 1_2375000)],
            lot: map![
                &e,
                (underlying_0.clone(), 30_5595329),
                (underlying_1.clone(), 1_5395739)
            ],
            block: 176,
        };
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        let positions: Positions = Positions {
            collateral: map![
                &e,
                (reserve_config_0.index, 90_9100000),
                (reserve_config_1.index, 04_5800000),
            ],
            liabilities: map![&e, (reserve_config_2.index, 02_7500000),],
            supply: map![&e],
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);
            //scale up modifiers
            e.ledger().set(LedgerInfo {
                timestamp: 12345 + 200 * 5,
                protocol_version: 22,
                sequence_number: 176 + 200,
                network_id: Default::default(),
                base_reserve: 10,
                min_temp_entry_ttl: 17280,
                min_persistent_entry_ttl: 17280,
                max_entry_ttl: 9999999,
            });
            let mut pool = Pool::load(&e);
            let mut frodo_state = User::load(&e, &frodo);
            fill_user_liq_auction(&e, &mut pool, &mut auction_data, &samwise, &mut frodo_state);
            let samwise_positions = storage::get_user_positions(&e, &samwise);
            let samwise_hf =
                PositionData::calculate_from_positions(&e, &mut pool, &samwise_positions)
                    .as_health_factor(&e);
            assert_eq!(samwise_hf, 1_1458977);
        });
    }

    #[test]
    fn test_fill_user_liquidation_auction_empty_bid() {
        let e = Env::default();

        e.mock_all_auths();
        e.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 175,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 17280,
            min_persistent_entry_ttl: 17280,
            max_entry_ttl: 9999999,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let frodo = Address::generate(&e);

        let pool_address = create_pool(&e);

        let (oracle_address, oracle_client) = testutils::create_mock_oracle(&e);

        // creating reserves for a pool exhausts the budget
        e.cost_estimate().budget().reset_unlimited();
        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_0, mut reserve_data_0) = testutils::default_reserve_meta();
        reserve_data_0.last_time = 12345;
        reserve_data_0.b_rate = 1_100_000_000_000;
        reserve_config_0.c_factor = 0_8500000;
        reserve_config_0.l_factor = 0_9000000;
        reserve_config_0.index = 0;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_0,
            &reserve_config_0,
            &reserve_data_0,
        );

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_1, mut reserve_data_1) = testutils::default_reserve_meta();
        reserve_data_1.b_rate = 1_200_000_000_000;
        reserve_config_1.c_factor = 0_7500000;
        reserve_config_1.l_factor = 0_7500000;
        reserve_data_1.last_time = 12345;
        reserve_config_1.index = 1;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_1,
            &reserve_config_1,
            &reserve_data_1,
        );

        let (underlying_2, reserve_2_asset) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config_2, reserve_data_2) = testutils::default_reserve_meta();
        reserve_config_2.c_factor = 0_0000000;
        reserve_config_2.l_factor = 0_7000000;
        reserve_config_2.index = 2;
        testutils::create_reserve(
            &e,
            &pool_address,
            &underlying_2,
            &reserve_config_2,
            &reserve_data_2,
        );

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![
                &e,
                Asset::Stellar(underlying_0.clone()),
                Asset::Stellar(underlying_1.clone()),
                Asset::Stellar(underlying_2.clone()),
            ],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 2_0000000, 4_0000000, 50_0000000]);

        reserve_2_asset.mint(&frodo, &0_8000000);
        reserve_2_asset.approve(&frodo, &pool_address, &i128::MAX, &1000000);

        let mut auction_data = AuctionData {
            bid: map![&e],
            lot: map![
                &e,
                (underlying_0.clone(), 30_5595329),
                (underlying_1.clone(), 1_5395739)
            ],
            block: 176,
        };
        let pool_config = PoolConfig {
            oracle: oracle_address,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        let positions: Positions = Positions {
            collateral: map![
                &e,
                (reserve_config_0.index, 90_9100000),
                (reserve_config_1.index, 04_5800000),
            ],
            liabilities: map![&e, (reserve_config_2.index, 02_7500000),],
            supply: map![&e],
        };
        e.as_contract(&pool_address, || {
            storage::set_user_positions(&e, &samwise, &positions);
            storage::set_pool_config(&e, &pool_config);

            e.ledger().set(LedgerInfo {
                timestamp: 12345 + 200 * 5,
                protocol_version: 22,
                sequence_number: 176 + 200,
                network_id: Default::default(),
                base_reserve: 10,
                min_temp_entry_ttl: 17280,
                min_persistent_entry_ttl: 17280,
                max_entry_ttl: 9999999,
            });
            let mut pool = Pool::load(&e);
            let mut frodo_state = User::load(&e, &frodo);
            fill_user_liq_auction(&e, &mut pool, &mut auction_data, &samwise, &mut frodo_state);
            let frodo_positions = frodo_state.positions;
            assert_eq!(
                frodo_positions
                    .collateral
                    .get(reserve_config_0.index)
                    .unwrap(),
                30_5595329
            );
            assert_eq!(
                frodo_positions
                    .collateral
                    .get(reserve_config_1.index)
                    .unwrap(),
                1_5395739
            );
            assert_eq!(frodo_positions.liabilities.len(), 0);
            let samwise_positions = storage::get_user_positions(&e, &samwise);
            assert_eq!(
                samwise_positions
                    .collateral
                    .get(reserve_config_0.index)
                    .unwrap(),
                90_9100000 - 30_5595329
            );
            assert_eq!(
                samwise_positions
                    .collateral
                    .get(reserve_config_1.index)
                    .unwrap(),
                04_5800000 - 1_5395739
            );
            assert_eq!(
                samwise_positions
                    .liabilities
                    .get(reserve_config_2.index)
                    .unwrap(),
                02_7500000 - 0
            );
        });
    }
}
