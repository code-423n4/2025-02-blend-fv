pub(crate) mod summaries;
pub(crate) mod mocks;

pub(crate) mod pool_rules;
pub(crate) mod withdraw_rules;
pub(crate) mod fund_management_rules;
pub(crate) mod deposit_rules;
pub(crate) mod user_rules;

use soroban_sdk::{Env, IntoVal, TryFromVal, Val, Vec};
use cvlr::nondet::*;
use cvlr_soroban::nondet_vec;

pub enum GhostBool {
    UnInit,
    Init (bool)
}

impl GhostBool {
    #[inline(never)]
    pub fn init(&mut self, b: bool) {
        *self = Self::Init(b);
    }


    #[inline(never)]
    pub fn set(&mut self, b: bool) {
        if let Self::Init(ref mut current) = self {
            *current = b;
        }
    }

    #[inline(never)]
    pub fn get(&self) -> bool {
        match self {
            Self::UnInit => cvlr::nondet(),
            Self::Init(v) => *v,
        }
    }
}

pub enum GhostI128 {
    UnInit,
    Init (i128)
}

impl GhostI128 {
    #[inline(never)]
    pub fn init(&mut self, v: i128) {
        *self = Self::Init(v);
    }


    #[inline(never)]
    pub fn set(&mut self, v: i128) {
        if let Self::Init(ref mut current) = self {
            *current = v;
        }
    }

    #[inline(never)]
    pub fn get(&self) -> i128 {
        match self {
            Self::UnInit => cvlr::nondet(),
            Self::Init(v) => *v,
        }
    }
}

pub enum GhostVec<T> {
    UnInit,
    Init(Vec<T>),
}

impl<T: Clone + Nondet+IntoVal<Env, Val>+TryFromVal<Env, Val>> GhostVec<T> {
    #[inline(never)]
    pub fn init(&mut self, vec: Vec<T>) {
        *self = Self::Init(vec);
    }

    #[inline(never)]
    pub fn set(&mut self, new_vec: Vec<T>) {
        *self = Self::Init(new_vec);
    }

    #[inline(never)]
    pub fn get(&self) -> Vec<T> {
        match self {
            Self::UnInit => nondet_vec(),
            Self::Init(ref vec) => vec.clone()
        }
    }
}
