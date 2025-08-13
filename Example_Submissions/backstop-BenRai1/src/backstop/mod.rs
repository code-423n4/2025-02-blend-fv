mod deposit;
pub use deposit::execute_deposit;

mod fund_management;
pub use fund_management::{execute_donate, execute_draw};

mod withdrawal;
pub use withdrawal::{execute_dequeue_withdrawal, execute_queue_withdrawal, execute_withdraw};

mod pool;
pub use pool::{
    load_pool_backstop_data, require_is_from_pool_factory, require_pool_above_threshold,
    PoolBackstopData, PoolBalance, 
};

mod user;
pub use user::{UserBalance, Q4W};

//@audit -------------- ADDED START -----------------------------------------


use cvlr::nondet::*;
pub enum GhostMap<K, V> { 
    UnInit,
    Init { k: K, v: V }
}

impl <K: Clone + Eq, V: Nondet + Clone> GhostMap<K, V> {
    #[inline(never)]
    pub fn init(&mut self, k: &K, v: V) {
        *self = Self::Init { k: k.clone(), v: v.clone() };
    }

    #[inline(never)]
    pub fn set(&mut self, k: &K, v: V) {
        match self {
            Self::Init { k: my_k, v: my_v} =>
                if k == my_k {
                    *my_v = v
                }
            _ => {}
        }
    }

    #[inline(never)]
    pub fn get(&self, k: &K) -> V {
        match self {
            Self::UnInit => V::nondet(),
            Self::Init { k: my_k, v: my_v } => {
                if k == my_k {
                    my_v.clone()
                } else {
                    V::nondet()
                }
            }
        }
    }
}

//---------------- ADDED END ---------------------------
