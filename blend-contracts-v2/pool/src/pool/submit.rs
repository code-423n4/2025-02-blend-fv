use moderc3156::FlashLoanClient;
use sep_41_token::TokenClient;
use soroban_sdk::{panic_with_error, Address, Env, Map, Vec};

use crate::{events::PoolEvents, storage, AuctionType, PoolError};

use super::{
    actions::{build_actions_from_request, Actions, Request},
    health_factor::PositionData,
    pool::Pool,
    FlashLoan, Positions, User,
};

/// Execute a set of updates for a user against the pool.
///
/// ### Arguments
/// * from - The address of the user whose positions are being modified
/// * spender - The address of the user who is sending tokens to the pool
/// * to - The address of the user who is receiving tokens from the pool
/// * requests - A vec of requests to be processed
/// * use_allowance - A bool indicating if transfer_from is to be used
///
/// ### Panics
/// If the request is unable to be fully executed
pub fn execute_submit(
    e: &Env,
    from: &Address,
    spender: &Address,
    to: &Address,
    requests: Vec<Request>,
    use_allowance: bool,
) -> Positions {
    if from == &e.current_contract_address()
        || spender == &e.current_contract_address()
        || to == &e.current_contract_address()
    {
        panic_with_error!(e, &PoolError::BadRequest);
    }
    let mut pool = Pool::load(e);
    let mut from_state = User::load(e, from);

    let prev_positions_count = from_state.positions.effective_count();

    let actions = build_actions_from_request(e, &mut pool, &mut from_state, requests);

    validate_submit(
        e,
        &mut pool,
        &from_state,
        prev_positions_count,
        actions.check_health,
    );

    if use_allowance {
        handle_transfer_with_allowance(e, &actions, spender, to);
    } else {
        handle_transfers(e, &actions, spender, to);
    }

    // store updated info to ledger
    pool.store_cached_reserves(e);
    from_state.store(e);

    from_state.positions
}

/// Same as `execute_submit` but specifically made for performing a flash loan borrow before
/// the other submitted requests.
pub fn execute_submit_with_flash_loan(
    e: &Env,
    from: &Address,
    flash_loan: FlashLoan,
    requests: Vec<Request>,
) -> Positions {
    if from == &e.current_contract_address() {
        panic_with_error!(e, &PoolError::BadRequest);
    }
    let mut pool = Pool::load(e);
    let mut from_state = User::load(e, from);

    let prev_positions_count = from_state.positions.effective_count();

    // note: we add the flash loan liabilities before processing the other
    // requests.
    {
        let mut reserve = pool.load_reserve(e, &flash_loan.asset, true);
        let d_tokens_minted = reserve.to_d_token_up(e, flash_loan.amount);
        from_state.add_liabilities(e, &mut reserve, d_tokens_minted);
        reserve.require_utilization_below_max(e);

        PoolEvents::flash_loan(
            e,
            flash_loan.asset.clone(),
            from.clone(),
            flash_loan.contract.clone(),
            flash_loan.amount,
            d_tokens_minted,
        );
    }

    let actions = build_actions_from_request(e, &mut pool, &mut from_state, requests);

    // always check health since flash_borrow requires it
    validate_submit(e, &mut pool, &from_state, prev_positions_count, true);

    // we deal with the flashloan transfer before the others to allow the flash
    // loan to yield the repaid or supplied amount in the transfers.
    TokenClient::new(e, &flash_loan.asset).transfer(
        &e.current_contract_address(),
        &flash_loan.contract,
        &flash_loan.amount,
    );
    // calls the receiver contract with "from" as the caller
    FlashLoanClient::new(&e, &flash_loan.contract).exec_op(
        &from,
        &flash_loan.asset,
        &flash_loan.amount,
        &0,
    );

    // note: at this point, the pool has sum_by_asset(actions.flash_borrow.1) for each involed asset, but the user also has
    // increased liabilities. These will have to be either fully repaid by now in the requests following the flash borrow
    // or the user needs to have some previously added collateral to cover the borrow, i.e user is already healthy at this point,
    // we just have to make sure that they have the balances they are claiming to have through the transfers.

    handle_transfer_with_allowance(e, &actions, from, from);

    // store updated info to ledger
    pool.store_cached_reserves(e);
    from_state.store(e);

    from_state.positions
}

/// Validate submit results in a valid state for the pool and user.
///
/// ### Arguments
/// * pool - The pool state. Writes the oracle cache if oracle data is fetched.
/// * from_state - The user state for "from"
/// * prev_positions_count - The initial number of positions for "from"
/// * check_health - A bool indicating if the health factor should be checked
fn validate_submit(
    e: &Env,
    pool: &mut Pool,
    from_state: &User,
    prev_positions_count: u32,
    check_health: bool,
) {
    // Verify max positions haven't been exceeded
    pool.require_under_max(e, &from_state.positions, prev_positions_count);

    // Verify "from" does not have an active liquidation post requests
    if storage::has_auction(
        e,
        &(AuctionType::UserLiquidation as u32),
        &from_state.address,
    ) {
        panic_with_error!(e, PoolError::AuctionInProgress);
    }

    // panics if the new positions set does not meet the health factor requirement
    // min is 1.0000100 to prevent rounding errors
    if check_health && from_state.has_liabilities() {
        let position_data = PositionData::calculate_from_positions(e, pool, &from_state.positions);
        if position_data.is_hf_under(e, 1_0000100) {
            panic_with_error!(e, PoolError::InvalidHf);
        } else if position_data.collateral_base < pool.config.min_collateral {
            panic_with_error!(e, PoolError::MinCollateralNotMet);
        }
    }
}

fn handle_transfer_with_allowance(e: &Env, actions: &Actions, spender: &Address, to: &Address) {
    // map of token -> amount
    // amount can be negative:
    // pool owes when amount > 0
    // spender owes when amount < 0
    let mut net_balances: Map<Address, i128> = Map::new(e);

    for (token, amount) in actions.spender_transfer.iter() {
        net_balances.set(
            token.clone(),
            net_balances.get(token).unwrap_or_default() - amount,
        );
    }
    for (token, amount) in actions.pool_transfer.iter() {
        net_balances.set(
            token.clone(),
            net_balances.get(token).unwrap_or_default() + amount,
        );
    }

    for (address, amount) in net_balances {
        let token = TokenClient::new(e, &address);
        if amount < 0 {
            // transfer tokens from sender to pool
            token.transfer_from(
                &e.current_contract_address(),
                spender,
                &e.current_contract_address(),
                &amount.abs(),
            );
        } else if amount > 0 {
            // transfer tokens from pool to "to"
            token.transfer(&e.current_contract_address(), to, &amount);
        }
    }
}

fn handle_transfers(e: &Env, actions: &Actions, spender: &Address, to: &Address) {
    // transfer tokens from sender to pool
    for (address, amount) in actions.spender_transfer.iter() {
        TokenClient::new(e, &address).transfer(spender, &e.current_contract_address(), &amount);
    }

    // transfer tokens from pool to "to"
    for (address, amount) in actions.pool_transfer.iter() {
        TokenClient::new(e, &address).transfer(&e.current_contract_address(), to, &amount);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        storage::{self, PoolConfig},
        testutils, AuctionData, RequestType,
    };

    use super::*;
    use sep_40_oracle::testutils::Asset;
    use soroban_sdk::{
        map,
        testutils::{Address as _, Ledger, LedgerInfo},
        vec, Symbol,
    };

    #[test]
    fn test_submit() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let frodo = Address::generate(&e);
        let merry = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, underlying_1_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

        underlying_0_client.mint(&frodo, &16_0000000);

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
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 5_0000000]);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 2,
        };
        e.as_contract(&pool, || {
            e.mock_all_auths_allowing_non_root_auth();
            storage::set_pool_config(&e, &pool_config);

            let pre_pool_balance_0 = underlying_0_client.balance(&pool);
            let pre_pool_balance_1 = underlying_1_client.balance(&pool);

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::SupplyCollateral as u32,
                    address: underlying_0,
                    amount: 15_0000000,
                },
                Request {
                    request_type: RequestType::Borrow as u32,
                    address: underlying_1,
                    amount: 1_5000000,
                },
            ];
            let positions = execute_submit(&e, &samwise, &frodo, &merry, requests, false);

            assert_eq!(positions.liabilities.len(), 1);
            assert_eq!(positions.collateral.len(), 1);
            assert_eq!(positions.supply.len(), 0);
            assert_eq!(positions.collateral.get_unchecked(0), 14_9999884);
            assert_eq!(positions.liabilities.get_unchecked(1), 1_4999983);

            assert_eq!(
                underlying_0_client.balance(&pool),
                pre_pool_balance_0 + 15_0000000
            );
            assert_eq!(
                underlying_1_client.balance(&pool),
                pre_pool_balance_1 - 1_5000000
            );

            assert_eq!(underlying_0_client.balance(&frodo), 1_0000000);
            assert_eq!(underlying_1_client.balance(&merry), 1_5000000);
        });
    }

    #[test]
    fn test_submit_use_allowance() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let frodo = Address::generate(&e);
        let merry = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, underlying_1_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

        underlying_0_client.mint(&frodo, &15_0000000);

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
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 5_0000000]);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool, || {
            e.mock_all_auths_allowing_non_root_auth();
            storage::set_pool_config(&e, &pool_config);

            let pre_pool_balance_0 = underlying_0_client.balance(&pool);
            let pre_pool_balance_1 = underlying_1_client.balance(&pool);

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::SupplyCollateral as u32,
                    address: underlying_0.clone(),
                    amount: 15_0000000,
                },
                Request {
                    request_type: RequestType::Borrow as u32,
                    address: underlying_1,
                    amount: 1_5000000,
                },
            ];
            underlying_0_client.approve(&frodo, &pool, &15_0000000, &e.ledger().sequence());
            assert_eq!(underlying_0_client.allowance(&frodo, &pool), 15_0000000);

            let positions = execute_submit(&e, &samwise, &frodo, &merry, requests, true);

            assert_eq!(positions.liabilities.len(), 1);
            assert_eq!(positions.collateral.len(), 1);
            assert_eq!(positions.supply.len(), 0);
            assert_eq!(positions.collateral.get_unchecked(0), 14_9999884);
            assert_eq!(positions.liabilities.get_unchecked(1), 1_4999983);

            assert_eq!(
                underlying_0_client.balance(&pool),
                pre_pool_balance_0 + 15_0000000
            );
            assert_eq!(underlying_1_client.allowance(&frodo, &pool), 0);
            assert_eq!(
                underlying_1_client.balance(&pool),
                pre_pool_balance_1 - 1_5000000
            );

            assert_eq!(underlying_0_client.balance(&frodo), 0);
            assert_eq!(underlying_1_client.balance(&merry), 1_5000000);
        });

        underlying_0_client.mint(&frodo, &15_0000000);

        e.as_contract(&pool, || {
            e.mock_all_auths_allowing_non_root_auth();
            storage::set_pool_config(&e, &pool_config);

            let pre_pool_balance_0 = underlying_0_client.balance(&pool);

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::SupplyCollateral as u32,
                    address: underlying_0.clone(),
                    amount: 15_0000000,
                },
                Request {
                    request_type: RequestType::Borrow as u32,
                    address: underlying_0,
                    amount: 1_0000000,
                },
            ];
            underlying_0_client.approve(&frodo, &pool, &14_0000000, &e.ledger().sequence());
            assert_eq!(underlying_0_client.allowance(&frodo, &pool), 14_0000000);
            let positions = execute_submit(&e, &samwise, &frodo, &merry, requests, true);

            // new_allowance = old_allowance - (deposit - borrow)
            assert_eq!(underlying_0_client.allowance(&frodo, &pool), 0);

            assert_eq!(positions.liabilities.len(), 2);
            assert_eq!(positions.collateral.len(), 1);
            assert_eq!(positions.supply.len(), 0);

            assert_eq!(positions.collateral.get_unchecked(0), 29_9999768);
            assert_eq!(positions.liabilities.get_unchecked(1), 1_4999983);

            assert_eq!(
                underlying_0_client.balance(&pool),
                pre_pool_balance_0 + 14_0000000
            );

            assert_eq!(underlying_0_client.balance(&frodo), 1_0000000);
        });
    }

    #[test]
    fn test_submit_use_allowance_over_repay() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let frodo = Address::generate(&e);
        let merry = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, underlying_1_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

        underlying_0_client.mint(&frodo, &15_0000000);

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
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 5_0000000]);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool, || {
            e.mock_all_auths_allowing_non_root_auth();
            storage::set_pool_config(&e, &pool_config);

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::SupplyCollateral as u32,
                    address: underlying_0,
                    amount: 15_0000000,
                },
                Request {
                    request_type: RequestType::Borrow as u32,
                    address: underlying_1.clone(),
                    amount: 1_5000000,
                },
            ];
            underlying_0_client.approve(&frodo, &pool, &15_0000000, &e.ledger().sequence());
            assert_eq!(underlying_0_client.allowance(&frodo, &pool), 15_0000000);

            let positions = execute_submit(&e, &samwise, &frodo, &merry, requests, true);

            assert_eq!(positions.liabilities.len(), 1);
            assert_eq!(positions.collateral.len(), 1);
            assert_eq!(positions.supply.len(), 0);
            assert_eq!(positions.collateral.get_unchecked(0), 14_9999884);
            assert_eq!(positions.liabilities.get_unchecked(1), 1_4999983);

            underlying_1_client.mint(&frodo, &1_6000000);

            let pre_pool_balance_1 = underlying_1_client.balance(&pool);

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::Repay as u32,
                    address: underlying_1,
                    amount: 1_6000000,
                },
            ];
            underlying_1_client.approve(&frodo, &pool, &1_5000001, &e.ledger().sequence());
            assert_eq!(underlying_1_client.allowance(&frodo, &pool), 1_5000001);
            let positions = execute_submit(&e, &samwise, &frodo, &merry, requests, true);

            // new_allowance = old_allowance - repay
            assert_eq!(underlying_1_client.allowance(&frodo, &pool), 0);

            assert_eq!(positions.liabilities.len(), 0);
            assert_eq!(positions.collateral.len(), 1);
            assert_eq!(positions.supply.len(), 0);

            assert_eq!(positions.collateral.get_unchecked(0), 14_9999884);

            assert_eq!(
                underlying_1_client.balance(&pool),
                pre_pool_balance_1 + 1_5000001
            );

            assert_eq!(underlying_1_client.balance(&frodo), 999999);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #9)")]
    fn test_submit_use_allowance_no_allowance() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let frodo = Address::generate(&e);
        let merry = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

        underlying_0_client.mint(&frodo, &16_0000000);

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
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 5_0000000]);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 2,
        };

        e.as_contract(&pool, || {
            e.mock_all_auths_allowing_non_root_auth();
            storage::set_pool_config(&e, &pool_config);
            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::SupplyCollateral as u32,
                    address: underlying_0,
                    amount: 15_0000000,
                },
                Request {
                    request_type: RequestType::Borrow as u32,
                    address: underlying_1,
                    amount: 1_5000000,
                },
            ];

            execute_submit(&e, &samwise, &frodo, &merry, requests, true);
        });
    }
    #[test]
    fn test_submit_no_liabilities_does_not_load_oracle() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let frodo = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let oracle = Address::generate(&e); // will fail if executed against

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, underlying_1_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

        underlying_0_client.mint(&frodo, &16_0000000);
        underlying_1_client.mint(&frodo, &10_0000000);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 2,
        };
        e.as_contract(&pool, || {
            e.mock_all_auths_allowing_non_root_auth();
            storage::set_pool_config(&e, &pool_config);

            let pre_pool_balance_0 = underlying_0_client.balance(&pool);
            let pre_pool_balance_1 = underlying_1_client.balance(&pool);

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::SupplyCollateral as u32,
                    address: underlying_0,
                    amount: 15_0000000,
                },
                // force check_health to true
                Request {
                    request_type: RequestType::Borrow as u32,
                    address: underlying_1.clone(),
                    amount: 1_5000000,
                },
                Request {
                    request_type: RequestType::Repay as u32,
                    address: underlying_1,
                    amount: 1_5000001,
                },
            ];
            let positions = execute_submit(&e, &samwise, &frodo, &frodo, requests, false);

            assert_eq!(positions.liabilities.len(), 0);
            assert_eq!(positions.collateral.len(), 1);
            assert_eq!(positions.supply.len(), 0);
            assert_eq!(positions.collateral.get_unchecked(0), 14_9999884);

            assert_eq!(
                underlying_0_client.balance(&pool),
                pre_pool_balance_0 + 15_0000000
            );
            assert_eq!(
                underlying_1_client.balance(&pool),
                pre_pool_balance_1 + 1 // repayment rounded against user
            );

            assert_eq!(underlying_0_client.balance(&frodo), 1_0000000);
            assert_eq!(underlying_1_client.balance(&frodo), 10_0000000 - 1);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1205)")]
    fn test_submit_requires_healhty() {
        let e = Env::default();
        e.mock_all_auths();

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let frodo = Address::generate(&e);
        let merry = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

        underlying_0_client.mint(&frodo, &16_0000000);

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
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 5_0000000]);

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });
        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 2,
        };
        e.as_contract(&pool, || {
            storage::set_pool_config(&e, &pool_config);

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::SupplyCollateral as u32,
                    address: underlying_0,
                    amount: 15_0000000,
                },
                Request {
                    request_type: RequestType::Borrow as u32,
                    address: underlying_1,
                    amount: 1_7500000,
                },
            ];
            execute_submit(&e, &samwise, &frodo, &merry, requests, false);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1200)")]
    fn test_submit_from_is_not_self() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        underlying_0_client.mint(&samwise, &16_0000000);

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![&e, Asset::Stellar(underlying_0.clone())],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 1_0000000]);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 2,
        };
        e.as_contract(&pool, || {
            e.mock_all_auths_allowing_non_root_auth();
            storage::set_pool_config(&e, &pool_config);

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::SupplyCollateral as u32,
                    address: underlying_0,
                    amount: 15_0000000,
                },
            ];
            execute_submit(&e, &pool, &samwise, &samwise, requests, false);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1200)")]
    fn test_submit_spender_is_not_self() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        underlying_0_client.mint(&samwise, &16_0000000);

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![&e, Asset::Stellar(underlying_0.clone())],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 1_0000000]);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 2,
        };
        e.as_contract(&pool, || {
            e.mock_all_auths_allowing_non_root_auth();
            storage::set_pool_config(&e, &pool_config);

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::SupplyCollateral as u32,
                    address: underlying_0,
                    amount: 15_0000000,
                },
            ];
            execute_submit(&e, &samwise, &pool, &samwise, requests, false);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1200)")]
    fn test_submit_to_is_not_self() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        underlying_0_client.mint(&samwise, &16_0000000);

        oracle_client.set_data(
            &bombadil,
            &Asset::Other(Symbol::new(&e, "USD")),
            &vec![&e, Asset::Stellar(underlying_0.clone())],
            &7,
            &300,
        );
        oracle_client.set_price_stable(&vec![&e, 1_0000000]);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 2,
        };
        e.as_contract(&pool, || {
            e.mock_all_auths_allowing_non_root_auth();
            storage::set_pool_config(&e, &pool_config);

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::SupplyCollateral as u32,
                    address: underlying_0,
                    amount: 15_0000000,
                },
            ];
            execute_submit(&e, &samwise, &samwise, &pool, requests, false);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1208)")]
    fn test_submit_over_max_positions() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, underlying_1_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

        underlying_0_client.mint(&samwise, &10_0000000);
        underlying_1_client.mint(&samwise, &10_0000000);

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
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 5_0000000]);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 3,
        };
        let user_positions = Positions {
            liabilities: map![&e, (0, 1_0000000)],
            collateral: map![&e, (0, 15_0000000)],
            supply: map![&e],
        };
        e.as_contract(&pool, || {
            e.mock_all_auths_allowing_non_root_auth();
            storage::set_pool_config(&e, &pool_config);
            storage::set_user_positions(&e, &samwise, &user_positions);

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::Borrow as u32,
                    address: underlying_1.clone(),
                    amount: 1_5000000,
                },
                Request {
                    request_type: RequestType::SupplyCollateral as u32,
                    address: underlying_1,
                    amount: 1_0000000,
                },
            ];
            execute_submit(&e, &samwise, &samwise, &samwise, requests, false);
        });
    }

    #[test]
    fn test_submit_over_max_positions_decrease_allowed() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, underlying_1_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

        underlying_0_client.mint(&samwise, &10_0000000);
        underlying_1_client.mint(&samwise, &10_0000000);

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
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 5_0000000]);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 2,
        };
        let user_positions = Positions {
            liabilities: map![&e, (0, 1_0000000), (1, 1_0000000)],
            collateral: map![&e, (0, 15_0000000), (1, 15_0000000)],
            supply: map![&e],
        };
        e.as_contract(&pool, || {
            e.mock_all_auths_allowing_non_root_auth();
            storage::set_pool_config(&e, &pool_config);
            storage::set_user_positions(&e, &samwise, &user_positions);

            let pre_pool_balance_0 = underlying_0_client.balance(&pool);
            let pre_pool_balance_1 = underlying_1_client.balance(&pool);

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::SupplyCollateral as u32,
                    address: underlying_0,
                    amount: 1_0000000,
                },
                Request {
                    request_type: RequestType::Repay as u32,
                    address: underlying_1,
                    amount: 2_0000000,
                },
            ];
            let result = execute_submit(&e, &samwise, &samwise, &samwise, requests, false);

            assert_eq!(result.liabilities.len(), 1);
            assert_eq!(result.collateral.len(), 2);

            assert_eq!(
                underlying_0_client.balance(&pool),
                pre_pool_balance_0 + 1_0000000
            );
            assert_eq!(
                underlying_1_client.balance(&pool),
                pre_pool_balance_1 + 1_0000012
            );

            assert_eq!(underlying_0_client.balance(&samwise), 9_0000000);
            assert_eq!(
                underlying_1_client.balance(&samwise),
                10_0000000 - 1_0000012
            );
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1212)")]
    fn test_submit_with_ongoing_liquidation_blocked() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, underlying_1_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

        underlying_0_client.mint(&samwise, &10_0000000);
        underlying_1_client.mint(&samwise, &10_0000000);

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
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 5_0000000]);

        let auction_data = AuctionData {
            bid: map![&e, (underlying_0.clone(), 2_0000000)],
            lot: map![&e, (underlying_1.clone(), 2_0000000),],
            block: 1200,
        };
        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 2,
        };
        let user_positions = Positions {
            liabilities: map![&e, (0, 5_0000000)],
            collateral: map![&e, (1, 6_0000000)],
            supply: map![&e],
        };
        e.as_contract(&pool, || {
            e.mock_all_auths_allowing_non_root_auth();
            storage::set_pool_config(&e, &pool_config);
            storage::set_user_positions(&e, &samwise, &user_positions);
            storage::set_auction(
                &e,
                &(AuctionType::UserLiquidation as u32),
                &samwise,
                &auction_data,
            );

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::Repay as u32,
                    address: underlying_0,
                    amount: 4_0000000,
                },
            ];
            execute_submit(&e, &samwise, &samwise, &samwise, requests, false);
        });
    }

    #[test]
    fn test_submit_with_ongoing_liquidation_works_if_canceled() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, underlying_1_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

        underlying_0_client.mint(&samwise, &10_0000000);
        underlying_1_client.mint(&samwise, &10_0000000);

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
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 5_0000000]);

        let auction_data = AuctionData {
            bid: map![&e, (underlying_0.clone(), 2_0000000)],
            lot: map![&e, (underlying_1.clone(), 2_0000000),],
            block: 1200,
        };
        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 2,
        };
        let user_positions = Positions {
            liabilities: map![&e, (0, 5_0000000)],
            collateral: map![&e, (1, 6_0000000)],
            supply: map![&e],
        };
        e.as_contract(&pool, || {
            e.mock_all_auths_allowing_non_root_auth();
            storage::set_pool_config(&e, &pool_config);
            storage::set_user_positions(&e, &samwise, &user_positions);
            storage::set_auction(
                &e,
                &(AuctionType::UserLiquidation as u32),
                &samwise,
                &auction_data,
            );

            let pre_pool_balance_0 = underlying_0_client.balance(&pool);

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::Repay as u32,
                    address: underlying_0,
                    amount: 4_0000000,
                },
                Request {
                    request_type: RequestType::DeleteLiquidationAuction as u32,
                    address: samwise.clone(),
                    amount: 0,
                },
            ];
            let result = execute_submit(&e, &samwise, &samwise, &samwise, requests, false);

            assert_eq!(result.liabilities.len(), 1);
            assert_eq!(result.collateral.len(), 1);

            assert_eq!(result.collateral.get_unchecked(1), 6_0000000);
            assert_eq!(result.liabilities.get_unchecked(0), 1_0000046);

            assert_eq!(
                underlying_0_client.balance(&pool),
                pre_pool_balance_0 + 4_0000000
            );
            assert_eq!(
                underlying_0_client.balance(&samwise),
                10_0000000 - 4_0000000
            );

            assert!(!storage::has_auction(
                &e,
                &(AuctionType::UserLiquidation as u32),
                &samwise
            ));
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1224)")]
    fn test_submit_under_min_collateral_fails() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let frodo = Address::generate(&e);
        let merry = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, _) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

        underlying_0_client.mint(&frodo, &16_0000000);

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
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 5_0000000]);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 2,
        };
        e.as_contract(&pool, || {
            e.mock_all_auths_allowing_non_root_auth();
            storage::set_pool_config(&e, &pool_config);

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::SupplyCollateral as u32,
                    address: underlying_0,
                    amount: 0_9000000,
                },
                Request {
                    request_type: RequestType::Borrow as u32,
                    address: underlying_1,
                    amount: 0_01000000,
                },
            ];
            execute_submit(&e, &samwise, &frodo, &merry, requests, false);
        });
    }

    /***** submit_with_flash_loan *****/

    #[test]
    fn test_submit_with_flash_loan() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (flash_loan_receiver, _) = testutils::create_flashloan_receiver(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config, mut reserve_data) = testutils::default_reserve_meta();
        reserve_config.max_util = 9500000;
        reserve_data.b_supply = 100_0000000;
        reserve_data.d_supply = 50_0000000;
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, underlying_1_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

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
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 5_0000000]);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool, || {
            storage::set_pool_config(&e, &pool_config);

            underlying_1_client.mint(&samwise, &25_0000000);
            underlying_1_client.approve(&samwise, &pool, &100_0000000, &10000);

            let pre_pool_balance_0 = underlying_0_client.balance(&pool);
            let pre_pool_balance_1 = underlying_1_client.balance(&pool);

            // pool has 100 supplied and 50 borrowed for asset_0
            // -> max util is 95%
            let flash_loan: FlashLoan = FlashLoan {
                contract: flash_loan_receiver,
                asset: underlying_0,
                amount: 25_0000000,
            };

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::SupplyCollateral as u32,
                    address: underlying_1,
                    amount: 25_0000000,
                },
            ];
            let positions = execute_submit_with_flash_loan(&e, &samwise, flash_loan, requests);

            assert_eq!(positions.liabilities.len(), 1);
            assert_eq!(positions.collateral.len(), 1);
            assert_eq!(positions.supply.len(), 0);
            assert_eq!(positions.collateral.get_unchecked(1), 249999807);
            // actual is 24.999979375 - rounds up
            assert_eq!(positions.liabilities.get_unchecked(0), 249999794);

            assert_eq!(
                underlying_0_client.balance(&pool),
                pre_pool_balance_0 - 25_0000000
            );
            assert_eq!(
                underlying_1_client.balance(&pool),
                pre_pool_balance_1 + 25_0000000
            );

            assert_eq!(underlying_0_client.balance(&samwise), 25_0000000);
            assert_eq!(underlying_1_client.balance(&samwise), 0);

            // check allowance is used
            assert_eq!(
                underlying_1_client.allowance(&samwise, &pool),
                100_0000000 - 25_0000000
            );
        });
    }

    #[test]
    fn test_submit_with_flash_loan_process_flash_loan_first() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (flash_loan_receiver, _) = testutils::create_flashloan_receiver(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config, mut reserve_data) = testutils::default_reserve_meta();
        reserve_config.max_util = 9500000;
        reserve_data.b_supply = 100_0000000;
        reserve_data.d_supply = 50_0000000;
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, underlying_1_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

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
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 5_0000000]);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool, || {
            storage::set_pool_config(&e, &pool_config);

            underlying_0_client.mint(&samwise, &1_0000000);
            underlying_0_client.approve(&samwise, &pool, &100_0000000, &10000);

            let pre_pool_balance_0 = underlying_0_client.balance(&pool);
            let pre_pool_balance_1 = underlying_1_client.balance(&pool);

            // pool has 100 supplied and 50 borrowed for asset_0
            // -> max util is 95%
            let flash_loan: FlashLoan = FlashLoan {
                contract: flash_loan_receiver,
                asset: underlying_0.clone(),
                amount: 25_0000000,
            };

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::Repay as u32,
                    address: underlying_0,
                    amount: 25_0000010,
                },
            ];
            let positions = execute_submit_with_flash_loan(&e, &samwise, flash_loan, requests);

            assert_eq!(positions.liabilities.len(), 0);
            assert_eq!(positions.collateral.len(), 0);
            assert_eq!(positions.supply.len(), 0);

            assert_eq!(underlying_0_client.balance(&pool), pre_pool_balance_0 + 1,);
            assert_eq!(underlying_1_client.balance(&pool), pre_pool_balance_1,);

            // rounding causes 1 stroops to be lost
            assert_eq!(underlying_0_client.balance(&samwise), 0_9999999);
            assert_eq!(underlying_1_client.balance(&samwise), 0);

            // check allowance is used
            assert_eq!(
                underlying_0_client.allowance(&samwise, &pool),
                100_0000000 - 25_0000001
            );
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1205)")]
    fn test_submit_with_flash_loan_checks_health() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (flash_loan_receiver, _) = testutils::create_flashloan_receiver(&e);

        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config, mut reserve_data) = testutils::default_reserve_meta();
        reserve_config.max_util = 9500000;
        reserve_data.b_supply = 100_0000000;
        reserve_data.d_supply = 50_0000000;
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, underlying_1_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

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
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 5_0000000]);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool, || {
            storage::set_pool_config(&e, &pool_config);

            underlying_1_client.mint(&samwise, &25_0000000);
            underlying_1_client.approve(&samwise, &pool, &100_0000000, &10000);

            // pool has 100 supplied and 50 borrowed for asset_0
            // -> max util is 95%
            let flash_loan: FlashLoan = FlashLoan {
                contract: flash_loan_receiver,
                asset: underlying_0,
                amount: 25_0000000,
            };

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::SupplyCollateral as u32,
                    address: underlying_1,
                    amount: 8_0000000,
                },
            ];
            execute_submit_with_flash_loan(&e, &samwise, flash_loan, requests);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1207)")]
    fn test_submit_with_flash_loan_checks_max_util() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (flash_loan_receiver, _) = testutils::create_flashloan_receiver(&e);

        let (underlying_0, _) = testutils::create_token_contract(&e, &bombadil);
        let (mut reserve_config, mut reserve_data) = testutils::default_reserve_meta();
        reserve_config.max_util = 9500000;
        reserve_data.b_supply = 100_0000000;
        reserve_data.d_supply = 50_0000000;
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, underlying_1_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

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
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 5_0000000]);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 4,
        };
        e.as_contract(&pool, || {
            storage::set_pool_config(&e, &pool_config);

            underlying_1_client.mint(&samwise, &50_0000000);
            underlying_1_client.approve(&samwise, &pool, &100_0000000, &10000);

            // pool has 100 supplied and 50 borrowed for asset_0
            // -> max util is 95%
            let flash_loan: FlashLoan = FlashLoan {
                contract: flash_loan_receiver,
                asset: underlying_0,
                amount: 46_0000000,
            };

            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::SupplyCollateral as u32,
                    address: underlying_1,
                    amount: 50_0000000,
                },
            ];
            execute_submit_with_flash_loan(&e, &samwise, flash_loan, requests);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1208)")]
    fn test_submit_with_flash_loan_over_max_positions() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (flash_loan_receiver, _) = testutils::create_flashloan_receiver(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, underlying_1_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

        underlying_0_client.mint(&samwise, &10_0000000);
        underlying_0_client.approve(&samwise, &pool, &10_0000000, &100000);
        underlying_1_client.mint(&samwise, &10_0000000);
        underlying_1_client.approve(&samwise, &pool, &10_0000000, &100000);

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
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 5_0000000]);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 3,
        };
        let user_positions = Positions {
            liabilities: map![&e, (1, 1_0000000)],
            collateral: map![&e, (0, 15_0000000)],
            supply: map![&e],
        };
        e.as_contract(&pool, || {
            e.mock_all_auths_allowing_non_root_auth();
            storage::set_pool_config(&e, &pool_config);
            storage::set_user_positions(&e, &samwise, &user_positions);

            let flash_loan: FlashLoan = FlashLoan {
                contract: flash_loan_receiver,
                asset: underlying_0,
                amount: 1_0000000,
            };
            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::SupplyCollateral as u32,
                    address: underlying_1,
                    amount: 2_0000000,
                },
            ];
            execute_submit_with_flash_loan(&e, &samwise, flash_loan, requests);
        });
    }

    #[test]
    fn test_submit_with_flash_loan_over_max_positions_decrease_allowed() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (flash_loan_receiver, _) = testutils::create_flashloan_receiver(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, underlying_1_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

        underlying_0_client.mint(&samwise, &10_0000000);
        underlying_0_client.approve(&samwise, &pool, &10_0000000, &100000);
        underlying_1_client.mint(&samwise, &10_0000000);
        underlying_1_client.approve(&samwise, &pool, &10_0000000, &100000);

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
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 5_0000000]);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 2,
        };
        let user_positions = Positions {
            liabilities: map![&e, (0, 1_0000000), (1, 1_0000000)],
            collateral: map![&e, (0, 15_0000000), (1, 15_0000000)],
            supply: map![&e],
        };
        e.as_contract(&pool, || {
            e.mock_all_auths_allowing_non_root_auth();
            storage::set_pool_config(&e, &pool_config);
            storage::set_user_positions(&e, &samwise, &user_positions);

            let pre_pool_balance_0 = underlying_0_client.balance(&pool);
            let pre_pool_balance_1 = underlying_1_client.balance(&pool);

            let flash_loan: FlashLoan = FlashLoan {
                contract: flash_loan_receiver,
                asset: underlying_0,
                amount: 1_0000000,
            };
            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::Repay as u32,
                    address: underlying_1,
                    amount: 2_0000000,
                },
            ];
            let result = execute_submit_with_flash_loan(&e, &samwise, flash_loan, requests);

            assert_eq!(result.liabilities.len(), 1);
            assert_eq!(result.collateral.len(), 2);

            assert_eq!(
                underlying_0_client.balance(&pool),
                pre_pool_balance_0 - 1_0000000
            );
            assert_eq!(
                underlying_1_client.balance(&pool),
                pre_pool_balance_1 + 1_0000012
            );

            assert_eq!(underlying_0_client.balance(&samwise), 11_0000000);
            assert_eq!(
                underlying_1_client.balance(&samwise),
                10_0000000 - 1_0000012
            );
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1212)")]
    fn test_submit_with_flash_loan_with_ongoing_liquidation_blocked() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (flash_loan_receiver, _) = testutils::create_flashloan_receiver(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, underlying_1_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

        underlying_0_client.mint(&samwise, &10_0000000);
        underlying_1_client.mint(&samwise, &10_0000000);

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
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 5_0000000]);

        let auction_data = AuctionData {
            bid: map![&e, (underlying_0.clone(), 2_0000000)],
            lot: map![&e, (underlying_1.clone(), 2_0000000),],
            block: 1200,
        };
        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 2,
        };
        let user_positions = Positions {
            liabilities: map![&e, (0, 5_0000000)],
            collateral: map![&e, (1, 6_0000000)],
            supply: map![&e],
        };
        e.as_contract(&pool, || {
            e.mock_all_auths_allowing_non_root_auth();
            storage::set_pool_config(&e, &pool_config);
            storage::set_user_positions(&e, &samwise, &user_positions);
            storage::set_auction(
                &e,
                &(AuctionType::UserLiquidation as u32),
                &samwise,
                &auction_data,
            );

            let flash_loan: FlashLoan = FlashLoan {
                contract: flash_loan_receiver,
                asset: underlying_0.clone(),
                amount: 1_0000000,
            };
            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::Repay as u32,
                    address: underlying_0,
                    amount: 4_5000000,
                },
            ];
            execute_submit_with_flash_loan(&e, &samwise, flash_loan, requests);
        });
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1224)")]
    fn test_submit_with_flash_loan_under_min_collateral_fails() {
        let e = Env::default();
        e.cost_estimate().budget().reset_unlimited();
        e.mock_all_auths_allowing_non_root_auth();

        e.ledger().set(LedgerInfo {
            timestamp: 600,
            protocol_version: 22,
            sequence_number: 1234,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 3110400,
        });

        let bombadil = Address::generate(&e);
        let samwise = Address::generate(&e);
        let pool = testutils::create_pool(&e);
        let (oracle, oracle_client) = testutils::create_mock_oracle(&e);

        let (flash_loan_receiver, _) = testutils::create_flashloan_receiver(&e);

        let (underlying_0, underlying_0_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_0, &reserve_config, &reserve_data);

        let (underlying_1, underlying_1_client) = testutils::create_token_contract(&e, &bombadil);
        let (reserve_config, reserve_data) = testutils::default_reserve_meta();
        testutils::create_reserve(&e, &pool, &underlying_1, &reserve_config, &reserve_data);

        underlying_0_client.mint(&samwise, &20_0000000);
        underlying_0_client.approve(&samwise, &pool, &20_0000000, &100000);
        underlying_1_client.mint(&samwise, &20_0000000);
        underlying_1_client.approve(&samwise, &pool, &20_0000000, &100000);

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
        oracle_client.set_price_stable(&vec![&e, 1_0000000, 5_0000000]);

        let pool_config = PoolConfig {
            oracle,
            min_collateral: 1_0000000,
            bstop_rate: 0_1000000,
            status: 0,
            max_positions: 2,
        };
        e.as_contract(&pool, || {
            e.mock_all_auths_allowing_non_root_auth();
            storage::set_pool_config(&e, &pool_config);

            let flash_loan: FlashLoan = FlashLoan {
                contract: flash_loan_receiver,
                asset: underlying_1.clone(),
                amount: 5_0000000,
            };
            let requests = vec![
                &e,
                Request {
                    request_type: RequestType::SupplyCollateral as u32,
                    address: underlying_0,
                    amount: 0_9000000,
                },
                Request {
                    request_type: RequestType::Repay as u32,
                    address: underlying_1,
                    amount: 4_9900000,
                },
            ];
            execute_submit_with_flash_loan(&e, &samwise, flash_loan, requests);
        });
    }
}
