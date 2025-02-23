use soroban_sdk::{Address, Env};

mod pool_factory_contract {
    soroban_sdk::contractimport!(
        file = "../target/wasm32-unknown-unknown/optimized/pool_factory.wasm"
    );
}
use pool_factory::{PoolFactoryClient, PoolInitMeta};

use mock_pool_factory::MockPoolFactory;

pub fn create_pool_factory<'a>(
    e: &Env,
    contract_id: &Address,
    wasm: bool,
    pool_init_meta: PoolInitMeta,
) -> PoolFactoryClient<'a> {
    if wasm {
        e.register_at(&contract_id, pool_factory_contract::WASM, (pool_init_meta,));
    } else {
        e.register_at(&contract_id, MockPoolFactory {}, (pool_init_meta,));
    }
    PoolFactoryClient::new(e, &contract_id)
}
