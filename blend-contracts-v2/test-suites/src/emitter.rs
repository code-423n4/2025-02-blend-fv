use soroban_sdk::{Address, Env};

use blend_contract_sdk::emitter::{Client as EmitterClient, WASM as EmitterWASM};

pub fn create_emitter<'a>(e: &Env) -> (Address, EmitterClient<'a>) {
    let contract_id = e.register(EmitterWASM, ());
    (contract_id.clone(), EmitterClient::new(e, &contract_id))
}
