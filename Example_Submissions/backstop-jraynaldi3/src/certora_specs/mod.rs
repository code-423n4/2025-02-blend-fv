pub(crate) mod summaries;
pub(crate) mod mocks;

pub(crate) mod pool_rules;
pub(crate) mod withdraw_rules;
pub(crate) mod fund_management_rules;
pub(crate) mod deposit_rules;
pub(crate) mod user_rules;

use crate::certora_specs::summaries::storage::{balance_of_user, balance_of_pool};
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
