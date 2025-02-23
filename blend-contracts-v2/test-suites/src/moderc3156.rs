use moderc3156_example::{
    FlashLoanReceiverModifiedERC3156, FlashLoanReceiverModifiedERC3156Client,
};
use soroban_sdk::{testutils::Address as _, Address, Env};

pub fn create_flashloan_receiver<'a>(
    e: &Env,
) -> (Address, FlashLoanReceiverModifiedERC3156Client<'a>) {
    let contract_id = Address::generate(e);
    e.register_at(&contract_id, FlashLoanReceiverModifiedERC3156 {}, ());

    (
        contract_id.clone(),
        FlashLoanReceiverModifiedERC3156Client::new(e, &contract_id),
    )
}
