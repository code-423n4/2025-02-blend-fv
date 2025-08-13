use soroban_sdk::{contracttype, Env};
use crate::backstop::Q4W;

// A simplified Vec implementation specifically for Q4W that can hold at most one element
#[derive(Clone)]
#[contracttype]
pub enum VecQ4W {
    Empty,
    One(Q4W),
}

impl VecQ4W {
    pub fn new(_e: &Env) -> Self {
        VecQ4W::Empty
    }

    pub fn len(&self) -> u32 {
        match self {
            VecQ4W::Empty => 0,
            VecQ4W::One(_) => 1,
        }
    }

    pub fn push_back(&mut self, value: Q4W) {
        match self {
            VecQ4W::Empty => *self = VecQ4W::One(value),
            VecQ4W::One(_) => panic!("VecQ4W already contains an element"),
        }
    }

    pub fn push_front(&mut self, value: Q4W) {
        self.push_back(value)
    }

    pub fn pop_back_unchecked(&mut self) -> Q4W {
        let result = match self {
            VecQ4W::Empty => panic!("VecQ4W is empty"),
            VecQ4W::One(ref val) => val.clone(),
        };
        *self = VecQ4W::Empty;
        result
    }

    pub fn pop_front_unchecked(&mut self) -> Q4W {
        self.pop_back_unchecked()
    }

    pub fn get(&self, index: u32) -> Option<Q4W> {
        match (self, index) {
            (VecQ4W::One(val), 0) => Some(val.clone()),
            _ => None,
        }
    }

    pub fn last(&self) -> Option<Q4W> {
        match self {
            VecQ4W::Empty => None,
            VecQ4W::One(val) => Some(val.clone()),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            VecQ4W::Empty => true,
            VecQ4W::One(_) => false,
        }
    }
    
    // Get the inner value directly
    pub fn inner_value(&self) -> Option<Q4W> {
        match self {
            VecQ4W::Empty => None,
            VecQ4W::One(q4w) => Some(q4w.clone()),
        }
    }
}

impl cvlr::nondet::Nondet for VecQ4W {
    fn nondet() -> Self {
        if cvlr::nondet() {
            VecQ4W::Empty
        } else {
            VecQ4W::One(cvlr::nondet())
        }
    }
}

pub fn vec_q4w_empty(_e: &Env) -> VecQ4W {
    VecQ4W::Empty
}