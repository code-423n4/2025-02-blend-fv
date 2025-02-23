#![cfg(test)]

use crate::{
    backstop::Q4W,
    dependencies::{CometClient, COMET_WASM},
    storage::{self},
    BackstopContract,
};

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    unwrap::UnwrapOptimized,
    vec, Address, BytesN, Env, IntoVal, Vec,
};

use sep_41_token::testutils::{MockTokenClient, MockTokenWASM};

use blend_contract_sdk::emitter::{Client as EmitterClient, WASM as EmitterWASM};
use mock_pool_factory::{MockPoolFactory, MockPoolFactoryClient, PoolInitMeta};

/// Create a backstop contract.
///
/// This sets random data in the constructor, so unit tests that
/// rely on any constructor data need to reset it.
pub(crate) fn create_backstop(e: &Env) -> Address {
    e.register(
        BackstopContract {},
        (
            Address::generate(e),
            Address::generate(e),
            Address::generate(e),
            Address::generate(e),
            Address::generate(e),
            Vec::<(Address, i128)>::new(e),
        ),
    )
}

pub(crate) fn create_token<'a>(e: &Env, admin: &Address) -> (Address, MockTokenClient<'a>) {
    let contract_address = Address::generate(e);
    e.register_at(&contract_address, MockTokenWASM, ());
    let client = MockTokenClient::new(e, &contract_address);
    client.initialize(&admin, &7, &"unit".into_val(e), &"test".into_val(e));
    (contract_address, client)
}

pub(crate) fn create_blnd_token<'a>(
    e: &Env,
    backstop: &Address,
    admin: &Address,
) -> (Address, MockTokenClient<'a>) {
    let (contract_address, client) = create_token(e, admin);

    e.as_contract(backstop, || {
        storage::set_blnd_token(e, &contract_address);
    });
    (contract_address, client)
}

pub(crate) fn create_usdc_token<'a>(
    e: &Env,
    backstop: &Address,
    admin: &Address,
) -> (Address, MockTokenClient<'a>) {
    let (contract_address, client) = create_token(e, admin);

    e.as_contract(backstop, || {
        storage::set_usdc_token(e, &contract_address);
    });
    (contract_address, client)
}

pub(crate) fn create_backstop_token<'a>(
    e: &Env,
    backstop: &Address,
    admin: &Address,
) -> (Address, MockTokenClient<'a>) {
    let (contract_address, client) = create_token(e, admin);

    e.as_contract(backstop, || {
        storage::set_backstop_token(e, &contract_address);
    });
    (contract_address, client)
}

// Not used to deploy pools in tests - filled with mock data
pub(crate) fn create_mock_pool_factory<'a>(
    e: &Env,
    backstop: &Address,
) -> (Address, MockPoolFactoryClient<'a>) {
    let pool_init_meta = PoolInitMeta {
        backstop: backstop.clone(),
        pool_hash: BytesN::<32>::from_array(&e, &[0u8; 32]),
        blnd_id: Address::generate(e),
    };
    let contract_address = e.register(MockPoolFactory {}, (pool_init_meta,));
    e.as_contract(backstop, || {
        storage::set_pool_factory(e, &contract_address);
    });
    (
        contract_address.clone(),
        MockPoolFactoryClient::new(e, &contract_address),
    )
}

pub(crate) fn create_emitter<'a>(
    e: &Env,
    backstop: &Address,
    backstop_token: &Address,
    blnd_token: &Address,
    emitter_last_distro: u64,
) -> (Address, EmitterClient<'a>) {
    let contract_address = e.register(EmitterWASM, ());

    let prev_timestamp = e.ledger().timestamp();
    e.ledger().set(LedgerInfo {
        timestamp: emitter_last_distro,
        protocol_version: 22,
        sequence_number: 0,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });
    e.as_contract(backstop, || {
        storage::set_emitter(e, &contract_address);
    });
    let client = EmitterClient::new(e, &contract_address);
    client.initialize(&blnd_token, &backstop, &backstop_token);
    e.ledger().set(LedgerInfo {
        timestamp: prev_timestamp,
        protocol_version: 22,
        sequence_number: 0,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });
    (contract_address.clone(), client)
}

/// Deploy a test Comet LP pool of 80% BLND / 20% USDC and set it as the backstop token.
///
/// Initializes the pool with the following settings:
/// - Swap fee: 0.3%
/// - BLND: 1,000
/// - USDC: 25
/// - Shares: 100
pub(crate) fn create_comet_lp_pool<'a>(
    e: &Env,
    admin: &Address,
    blnd_token: &Address,
    usdc_token: &Address,
) -> (Address, CometClient<'a>) {
    let contract_address = Address::generate(e);
    e.register_at(&contract_address, COMET_WASM, ());
    let client = CometClient::new(e, &contract_address);

    let blnd_client = MockTokenClient::new(e, blnd_token);
    let usdc_client = MockTokenClient::new(e, usdc_token);
    blnd_client.mint(&admin, &1_000_0000000);
    usdc_client.mint(&admin, &25_0000000);

    client.init(
        admin,
        &vec![e, blnd_token.clone(), usdc_token.clone()],
        &vec![e, 0_8000000, 0_2000000],
        &vec![e, 1_000_0000000, 25_0000000],
        &0_0030000,
    );

    (contract_address, client)
}

/// Deploy a test Comet LP pool of 80% BLND / 20% USDC and set it as the backstop token.
///
/// Initializes the pool with the following settings:
/// - Swap fee: 0.3%
/// - BLND: 100 * blnd_per_share
/// - USDC: 100 * usdc_per_share
/// - Shares: 100
pub(crate) fn create_comet_lp_pool_with_tokens_per_share<'a>(
    e: &Env,
    backstop: &Address,
    admin: &Address,
    blnd_token: &Address,
    blnd_per_share: i128,
    usdc_token: &Address,
    usdc_per_share: i128,
) -> (Address, CometClient<'a>) {
    let contract_address = Address::generate(e);
    e.register_at(&contract_address, COMET_WASM, ());
    let client = CometClient::new(e, &contract_address);

    let blnd_client = MockTokenClient::new(e, blnd_token);
    let usdc_client = MockTokenClient::new(e, usdc_token);
    let blnd_total = 100 * blnd_per_share;
    let usdc_total = 100 * usdc_per_share;
    blnd_client.mint(&admin, &blnd_total);
    usdc_client.mint(&admin, &usdc_total);

    // init seeds pool with 100 shares
    client.init(
        admin,
        &vec![e, blnd_token.clone(), usdc_token.clone()],
        &vec![e, 0_8000000, 0_2000000],
        &vec![e, blnd_total, usdc_total],
        &0_0030000,
    );

    e.as_contract(backstop, || {
        storage::set_backstop_token(e, &contract_address);
    });

    (contract_address, client)
}

/********** Comparison Helpers **********/

pub(crate) fn assert_eq_vec_q4w(actual: &Vec<Q4W>, expected: &Vec<Q4W>) {
    assert_eq!(actual.len(), expected.len());
    for index in 0..actual.len() {
        let actual_q4w = actual.get(index).unwrap_optimized();
        let expected_q4w = expected.get(index).unwrap_optimized();
        assert_eq!(actual_q4w.amount, expected_q4w.amount);
        assert_eq!(actual_q4w.exp, expected_q4w.exp);
    }
}
