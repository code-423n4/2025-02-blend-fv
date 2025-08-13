use soroban_sdk::{Env, Address};
use cvlr::cvlr_satisfy;

pub fn sanity(
    _e: &Env, 
    _pool: &Address, 
    _user: &Address, 
    _amount: i128,
    call_fn: impl FnOnce()
) {        
    call_fn();
    cvlr_satisfy!(true);
}
