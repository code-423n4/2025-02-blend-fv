use soroban_sdk::{Address, Env, Vec};

mod backstop_contract_wasm {
    soroban_sdk::contractimport!(file = "../target/wasm32-unknown-unknown/optimized/backstop.wasm");
}
use backstop::{BackstopClient, BackstopContract};

pub fn create_backstop<'a>(
    e: &Env,
    contract_id: &Address,
    wasm: bool,
    backstop_token: &Address,
    emitter: &Address,
    blnd_token: &Address,
    usdc_token: &Address,
    pool_factory: &Address,
    drop_list: &Vec<(Address, i128)>,
) -> BackstopClient<'a> {
    if wasm {
        e.register_at(
            contract_id,
            backstop_contract_wasm::WASM,
            (
                backstop_token,
                emitter,
                blnd_token,
                usdc_token,
                pool_factory,
                drop_list.clone(),
            ),
        );
    } else {
        e.register_at(
            contract_id,
            BackstopContract {},
            (
                backstop_token,
                emitter,
                blnd_token,
                usdc_token,
                pool_factory,
                drop_list.clone(),
            ),
        );
    }
    BackstopClient::new(e, &contract_id)
}
