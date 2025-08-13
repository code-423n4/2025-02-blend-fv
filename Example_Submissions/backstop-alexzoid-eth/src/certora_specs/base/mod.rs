pub(crate) mod log_state;

use soroban_sdk::Env;
use cvlr::cvlr_assume;

pub trait Call {
    fn call(&self, env: &Env);
}

#[macro_export]
macro_rules! pass_arg {
    ($arg:expr, Address) => { &$arg };
    ($arg:expr, PoolBackstopData) => { &$arg };
    ($arg:expr, $ty:ty) => { $arg };
}

#[macro_export]
macro_rules! make_callable {
    ($contract:ident, $call:ident, $($arg:ident : $ty:ty),*) => {
        paste::paste!(
            #[allow(non_camel_case_types)]
            pub struct [< call_ $call >] { $(pub $arg: $ty,)* } 
            impl Call for [< call_ $call >] {
                fn call(&self, env: &Env) {
                    $contract::$call(env, $(pass_arg!(self.$arg, $ty)),*);
                }
            }
        );
    };

    ($contract:ident, $call:ident, no_env, $($arg:ident : $ty:ty),*) => {
        paste::paste!(
            #[allow(non_camel_case_types)]
            pub struct [< call_ $call >] { $(pub $arg: $ty,)* }  
            impl Call for [< call_ $call >] {
                fn call(&self, _env: &Env) {
                    $contract::$call($(pass_arg!(self.$arg, $ty)),*);
                }
            }
        );
    };
}

pub fn clear_upper_bits(value: i128) {
    let i128_upper_bits = (value.abs() >> 63) as i64;
    cvlr_assume!(i128_upper_bits == 0);
}

#[macro_export]
macro_rules! init_verification {
    ($e:expr, $pb:expr, $ub:expr, $pool:expr, $user:expr, $amount:expr, $q4w_len:expr) => {
        
        #[cfg(feature = "certora_storage_ghost")] 
        {
            // Initialize ghost storage from rule parameters
            storage::initialize_ghost_maps($pb.clone(), $ub.clone());

            // Explicitly assume that reading from the ghost maps returns the expected values
            let read_pb = storage::get_pool_balance(&$e, &$pool);
            let read_ub = storage::get_user_balance(&$e, &$pool, &$user);
            cvlr_assume!(read_pb.shares == $pb.shares);
            cvlr_assume!(read_pb.tokens == $pb.tokens);
            cvlr_assume!(read_pb.q4w == $pb.q4w);
            cvlr_assume!(read_ub.shares == $ub.shares);
            cvlr_assume!(read_ub.q4w.len() == $ub.q4w.len());
            cvlr_assume!(read_ub.q4w.len() <= $q4w_len);

            // Bound inputs as i32
            cvlr_assume!($amount as i64 >= i32::MIN as i64 
                && $amount as i64 <= i32::MAX as i64);
            cvlr_assume!(read_pb.shares as i64 >= i32::MIN as i64 
                && read_pb.shares as i64 <= i32::MAX as i64);
            cvlr_assume!(read_pb.tokens as i64 >= i32::MIN as i64 
                && read_pb.tokens as i64 <= i32::MAX as i64);
            cvlr_assume!(read_pb.q4w as i64 >= i32::MIN as i64 
                && read_pb.q4w as i64 <= i32::MAX as i64);
            cvlr_assume!(read_ub.shares as i64 >= i32::MIN as i64 
                && read_ub.shares as i64 <= i32::MAX as i64);
            if read_ub.q4w.len() != 0 {
                let entry0 = read_ub.q4w.get(0).unwrap_optimized();
                cvlr_assume!(entry0.amount as i64 >= i32::MIN as i64 
                    && entry0.amount as i64 <= i32::MAX as i64);
                if read_ub.q4w.len() == 2 {
                    let entry1 = read_ub.q4w.get(1).unwrap_optimized();
                    cvlr_assume!(entry1.amount as i64 >= i32::MIN as i64 
                        && entry1.amount as i64 <= i32::MAX as i64);    
                }
            }

            // @note helps in i128 comparison problem
            clear_upper_bits($amount);
            clear_upper_bits(read_pb.shares);
            clear_upper_bits(read_pb.tokens);
            clear_upper_bits(read_pb.q4w);
            clear_upper_bits(read_ub.shares);    
            if read_ub.q4w.len() != 0 {
                let entry0 = read_ub.q4w.get(0).unwrap_optimized();
                clear_upper_bits(entry0.amount);
                if read_ub.q4w.len() == 2 {
                    let entry1 = read_ub.q4w.get(1).unwrap_optimized();
                    clear_upper_bits(entry1.amount);    
                }
            }
        }

        // Assume valid state
        cvlr_assume!($e.ledger().timestamp() != 0);
        cvlr_assume!(valid_state_pool_user($e.clone(), $pool.clone(), $user.clone()));
    };
}

#[macro_export]
macro_rules! parametric_rule {
    ($f:ident) => {
        paste::paste! {
            #[rule]
            pub fn [< $f _execute_deposit >](
                e: Env, 
                pb: PoolBalance, 
                ub: UserBalance, 
                from: Address, 
                pool_address: Address, 
                amount: i128
            ) {
                init_verification!(e, pb, ub, pool_address, from, amount, FV_MAX_Q4W_VEC_LEN);
                
                let call_fn = || { 
                    log_state_details(e.clone(), pool_address.clone(), from.clone(), amount);
                    backstop::execute_deposit(&e.clone(), &from.clone(), &pool_address.clone(), amount); 
                    log_state_details(e.clone(), pool_address.clone(), from.clone(), amount);
                };
                
                $f(&e, &pool_address, &from, amount, call_fn);
            }
            
            #[rule]
            pub fn [< $f _execute_donate >](
                e: Env, 
                pb: PoolBalance, 
                ub: UserBalance, 
                from: Address, 
                pool_address: Address, 
                amount: i128
            ) {
                init_verification!(e, pb, ub, pool_address, from, amount, FV_MAX_Q4W_VEC_LEN);
                
                let call_fn = || { 
                    log_state_details(e.clone(), pool_address.clone(), from.clone(), amount);
                    backstop::execute_donate(&e.clone(), &from.clone(), &pool_address.clone(), amount); 
                    log_state_details(e.clone(), pool_address.clone(), from.clone(), amount);
                };
                
                $f(&e, &pool_address, &from, amount, call_fn);
            }
            
            #[rule]
            pub fn [< $f _execute_draw >](
                e: Env, 
                pb: PoolBalance, 
                ub: UserBalance, 
                pool_address: Address, 
                amount: i128,
                to: Address
            ) {
                init_verification!(e, pb, ub, pool_address, to, amount, FV_MAX_Q4W_VEC_LEN);
                
                let call_fn = || { 
                    log_state_details(e.clone(), pool_address.clone(), to.clone(), amount);
                    backstop::execute_draw(&e.clone(), &pool_address.clone(), amount, &to.clone()); 
                    log_state_details(e.clone(), pool_address.clone(), to.clone(), amount);
                };
                
                $f(&e, &pool_address, &to, amount, call_fn);
            }
            
            #[rule]
            pub fn [< $f _execute_dequeue_withdrawal >](
                e: Env, 
                pb: PoolBalance, 
                ub: UserBalance, 
                from: Address, 
                pool_address: Address, 
                amount: i128
            ) {
                init_verification!(e, pb, ub, pool_address, from, amount, FV_MAX_Q4W_VEC_LEN);
                
                let call_fn = || { 
                    log_state_details(e.clone(), pool_address.clone(), from.clone(), amount);
                    backstop::execute_dequeue_withdrawal(&e.clone(), &from.clone(), &pool_address.clone(), amount); 
                    log_state_details(e.clone(), pool_address.clone(), from.clone(), amount);
                };
                
                $f(&e, &pool_address, &from, amount, call_fn);
            }
            
            #[rule]
            pub fn [< $f _execute_queue_withdrawal >](
                e: Env, 
                pb: PoolBalance, 
                ub: UserBalance, 
                from: Address, 
                pool_address: Address, 
                amount: i128
            ) {
                init_verification!(e, pb, ub, pool_address, from, amount, FV_MAX_Q4W_VEC_LEN);
                
                let call_fn = || { 
                    log_state_details(e.clone(), pool_address.clone(), from.clone(), amount);
                    backstop::execute_queue_withdrawal(&e.clone(), &from.clone(), &pool_address.clone(), amount); 
                    log_state_details(e.clone(), pool_address.clone(), from.clone(), amount);
                };
                
                $f(&e, &pool_address, &from, amount, call_fn);
            }
            
            #[rule]
            pub fn [< $f _execute_withdraw >](
                e: Env, 
                pb: PoolBalance, 
                ub: UserBalance, 
                from: Address, 
                pool_address: Address, 
                amount: i128
            ) {
                init_verification!(e, pb, ub, pool_address, from, amount, FV_MAX_Q4W_VEC_LEN);
                
                let call_fn = || { 
                    log_state_details(e.clone(), pool_address.clone(), from.clone(), amount);
                    backstop::execute_withdraw(&e.clone(), &from.clone(), &pool_address.clone(), amount); 
                    log_state_details(e.clone(), pool_address.clone(), from.clone(), amount);
                };
                
                $f(&e, &pool_address, &from, amount, call_fn);
            }
        }
    };
}

#[macro_export]
macro_rules! invariant_rule {
    ($inv:ident) => {
        paste::paste! {
            fn [< $inv _helper >](
                e: Env,
                pb: PoolBalance,
                ub: UserBalance,
                pool: Address,
                user: Address,
                amount: i128,
                call_fn: impl FnOnce()
            ) {
                init_verification!(e, pb, ub, pool, user, amount, FV_MAX_Q4W_VEC_LEN);

                cvlr_assume!($inv(e.clone(), pool.clone(), user.clone()));
                
                log_state_details(e.clone(), pool.clone(), user.clone(), amount);
                call_fn();
                log_state_details(e.clone(), pool.clone(), user.clone(), amount);

                cvlr_assert!($inv(e.clone(), pool.clone(), user.clone()));
            }
            
            #[rule]
            pub fn [< $inv _execute_deposit >](
                e: Env, 
                pb: PoolBalance, 
                ub: UserBalance, 
                from: Address, 
                pool_address: Address, 
                amount: i128
            ) {
                let e_clone = e.clone();
                let from_clone = from.clone();
                let pool_clone = pool_address.clone();
                
                [< $inv _helper >](
                    e, pb, ub, pool_address, from, amount, 
                    || { backstop::execute_deposit(&e_clone, &from_clone, &pool_clone, amount); }
                );
            }
            
            #[rule]
            pub fn [< $inv _execute_donate >](
                e: Env, 
                pb: PoolBalance, 
                ub: UserBalance, 
                from: Address, 
                pool_address: Address, 
                amount: i128
            ) {
                let e_clone = e.clone();
                let from_clone = from.clone();
                let pool_clone = pool_address.clone();
                
                [< $inv _helper >](
                    e, pb, ub, pool_address, from, amount, 
                    || { backstop::execute_donate(&e_clone, &from_clone, &pool_clone, amount); }
                );
            }
            
            #[rule]
            pub fn [< $inv _execute_draw >](
                e: Env, 
                pb: PoolBalance, 
                ub: UserBalance, 
                pool_address: Address, 
                amount: i128,
                to: Address
            ) {
                let e_clone = e.clone();
                let to_clone = to.clone();
                let pool_clone = pool_address.clone();
                
                [< $inv _helper >](
                    e, pb, ub, pool_address, to, amount, 
                    || { backstop::execute_draw(&e_clone, &pool_clone, amount, &to_clone); }
                );
            }
            
            #[rule]
            pub fn [< $inv _execute_dequeue_withdrawal >](
                e: Env, 
                pb: PoolBalance, 
                ub: UserBalance, 
                from: Address, 
                pool_address: Address, 
                amount: i128
            ) {
                let e_clone = e.clone();
                let from_clone = from.clone();
                let pool_clone = pool_address.clone();
                
                [< $inv _helper >](
                    e, pb, ub, pool_address, from, amount, 
                    || { backstop::execute_dequeue_withdrawal(&e_clone, &from_clone, &pool_clone, amount); }
                );
            }
            
            #[rule]
            pub fn [< $inv _execute_queue_withdrawal >](
                e: Env, 
                pb: PoolBalance, 
                ub: UserBalance, 
                from: Address, 
                pool_address: Address, 
                amount: i128
            ) {
                let e_clone = e.clone();
                let from_clone = from.clone();
                let pool_clone = pool_address.clone();
                
                [< $inv _helper >](
                    e, pb, ub, pool_address, from, amount, 
                    || { backstop::execute_queue_withdrawal(&e_clone, &from_clone, &pool_clone, amount); }
                );
            }
            
            #[rule]
            pub fn [< $inv _execute_withdraw >](
                e: Env, 
                pb: PoolBalance, 
                ub: UserBalance, 
                from: Address, 
                pool_address: Address, 
                amount: i128
            ) {
                let e_clone = e.clone();
                let from_clone = from.clone();
                let pool_clone = pool_address.clone();
                
                [< $inv _helper >](
                    e, pb, ub, pool_address, from, amount, 
                    || { backstop::execute_withdraw(&e_clone, &from_clone, &pool_clone, amount); }
                );
            }
        }
    };
}

// GhostMap for storage_ghost, inspired by https://github.com/Certora/reflector-subscription-contract/blob/51944577dc4536e9cf9711db6e125fe1e2254054/src/certora_specs/mod.rs
use cvlr::nondet::*;

pub enum GhostMap<V> {
    UnInit,
    Init { v: V }
}

impl<V: Nondet + Clone> GhostMap<V> {
    #[inline(never)]
    pub fn init(&mut self, v: V) {
        *self = Self::Init { v };
    }

    #[inline(never)]
    pub fn set(&mut self, v: V) {
        match self {
            Self::Init { v: my_v } => {
                *my_v = v;
            },
            Self::UnInit => {
                *self = Self::Init { v };
            }
        }
    }

    #[inline(never)]
    pub fn get(&self) -> V {
        match self {
            Self::UnInit => V::nondet(),
            Self::Init { v: my_v } => my_v.clone()
        }
    }

    #[inline(never)]
    pub fn is_uninit(&self) -> bool {
        match self {
            Self::UnInit => true,
            Self::Init { .. } => false,
        }
    }
}
