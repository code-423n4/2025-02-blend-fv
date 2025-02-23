#![cfg(test)]
use std::i128;

use pool::{Request, RequestType};
use soroban_sdk::{testutils::Address as AddressTestTrait, vec, Address, Vec};
use test_suites::{
    create_fixture_with_data,
    test_fixture::{TokenIndex, SCALAR_12, SCALAR_7},
};

#[test]
fn test_pool_overflow() {
    let fixture = create_fixture_with_data(false);
    let pool_fixture = &fixture.pools[0];

    // overflow can occur when a user's oracle balance is >i128::MAX
    // during a health check

    let weth = &fixture.tokens[TokenIndex::WETH];
    let stable = &fixture.tokens[TokenIndex::STABLE];

    // check pool balance to ensure balance does not overflow first
    // 50% util, leave some room for interest accumulation
    let pool_weth_balance = weth.balance(&pool_fixture.pool.address);
    let max_weth_deposit = i128::MAX - 2 * pool_weth_balance - 5 * SCALAR_12;

    // under collateral cap for stable
    let max_stable_deposit = 999_000_000_000_000000;

    // Create a user
    let samwise = Address::generate(&fixture.env);
    weth.mint(&samwise, &max_weth_deposit);
    stable.mint(&samwise, &(2 * max_stable_deposit));

    // deposit tokens into large deposit
    let deposit_requests = vec![
        &fixture.env,
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: weth.address.clone(),
            amount: max_weth_deposit,
        },
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: stable.address.clone(),
            amount: max_stable_deposit,
        },
    ];
    pool_fixture
        .pool
        .submit(&samwise, &samwise, &samwise, &deposit_requests);

    // allow some time to pass
    fixture.jump_with_sequence(60 * 60 * 24);

    // validate a health check would overflow if it is >i128::MAX
    let borrow_request = vec![
        &fixture.env,
        Request {
            request_type: RequestType::Borrow as u32,
            address: stable.address.clone(),
            amount: SCALAR_7,
        },
    ];
    let borrow_res = pool_fixture
        .pool
        .try_submit(&samwise, &samwise, &samwise, &borrow_request);
    assert_eq!(borrow_res.is_err(), true);

    // validate the funds can still be withdrawn
    let withdraw_requests = vec![
        &fixture.env,
        Request {
            request_type: RequestType::WithdrawCollateral as u32,
            address: weth.address.clone(),
            amount: i128::MAX,
        },
    ];
    pool_fixture
        .pool
        .submit(&samwise, &samwise, &samwise, &withdraw_requests);
    // util is ~=0% so assert at most 1 stroop was lost to rounding
    assert!(weth.balance(&samwise) >= max_weth_deposit - 1);
}

// This test ensures that an accessible underflow in the auction flow cannot be hit due to the overflow-checks flag being set
// Without this flag set, filling an auction on the same block it's started would cause an underflow
#[test]
#[should_panic(expected = "Error(WasmVm, InvalidAction)")]
fn test_auction_underflow_panics() {
    let fixture = create_fixture_with_data(true);
    let frodo = fixture.users.get(0).unwrap();
    let pool_fixture = &fixture.pools[0];

    // Create a user
    let samwise = Address::generate(&fixture.env); //sam will be supplying XLM and borrowing STABLE

    // Mint users tokens
    fixture.tokens[TokenIndex::XLM].mint(&samwise, &(500_000 * SCALAR_7));

    // Supply and borrow sam tokens
    let sam_requests: Vec<Request> = vec![
        &fixture.env,
        Request {
            request_type: RequestType::SupplyCollateral as u32,
            address: fixture.tokens[TokenIndex::XLM].address.clone(),
            amount: 6_000 * SCALAR_7,
        },
        Request {
            request_type: RequestType::Borrow as u32,
            address: fixture.tokens[TokenIndex::STABLE].address.clone(),
            amount: 200 * 10i128.pow(6),
        },
    ];
    pool_fixture
        .pool
        .submit(&samwise, &samwise, &samwise, &sam_requests);

    //tank xlm price
    fixture.oracle.set_price_stable(&vec![
        &fixture.env,
        1000_0000000, // eth
        1_0000000,    // usdc
        0_0000100,    // xlm
        1_0000000,    // stable
    ]);

    // liquidate user
    let liq_pct = 100;
    let auction_data_2 = pool_fixture.pool.new_auction(
        &0,
        &samwise,
        &vec![
            &fixture.env,
            fixture.tokens[TokenIndex::STABLE].address.clone(),
        ],
        &vec![
            &fixture.env,
            fixture.tokens[TokenIndex::XLM].address.clone(),
        ],
        &liq_pct,
    );

    let usdc_bid_amount = auction_data_2
        .bid
        .get_unchecked(fixture.tokens[TokenIndex::STABLE].address.clone());

    //fill user liquidation
    let fill_requests = vec![
        &fixture.env,
        Request {
            request_type: RequestType::FillUserLiquidationAuction as u32,
            address: samwise.clone(),
            amount: 1,
        },
        Request {
            request_type: RequestType::Repay as u32,
            address: fixture.tokens[TokenIndex::STABLE].address.clone(),
            amount: usdc_bid_amount,
        },
    ];
    pool_fixture
        .pool
        .submit(&frodo, &frodo, &frodo, &fill_requests);
}
