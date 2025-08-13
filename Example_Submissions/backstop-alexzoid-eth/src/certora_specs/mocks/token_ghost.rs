use soroban_sdk::Address;

#[derive(Clone, Debug, PartialEq)]
pub enum TokenOpType {
    None,
    Transfer,
    TransferFrom,
}

#[derive(Clone, Debug)]
pub struct TokenOp {
    pub op_type: TokenOpType,
    pub from: Address,
    pub to: Address,
    pub amount: i128,
}

pub enum GhostTokenOps {
    UnInit,
    Init { last_op: Option<TokenOp> }
}

pub(crate) static mut GHOST_TOKEN_OPS: GhostTokenOps = GhostTokenOps::UnInit;

impl GhostTokenOps {
    #[inline(never)]
    pub fn init(&mut self) {
        *self = Self::Init { last_op: None };
    }

    #[inline(never)]
    pub fn record_transfer(&mut self, from: &Address, to: &Address, amount: i128) {
        match self {
            Self::Init { last_op } => {
                *last_op = Some(TokenOp {
                    op_type: TokenOpType::Transfer,
                    from: from.clone(),
                    to: to.clone(),
                    amount,
                });
            },
            Self::UnInit => {
                *self = Self::Init { 
                    last_op: Some(TokenOp {
                        op_type: TokenOpType::Transfer,
                        from: from.clone(),
                        to: to.clone(),
                        amount,
                    })
                };
            }
        }
    }

    #[inline(never)]
    pub fn record_transfer_from(&mut self, spender: &Address, from: &Address, to: &Address, amount: i128) {
        match self {
            Self::Init { last_op } => {
                *last_op = Some(TokenOp {
                    op_type: TokenOpType::TransferFrom,
                    from: from.clone(),
                    to: to.clone(),
                    amount,
                });
            },
            Self::UnInit => {
                *self = Self::Init { 
                    last_op: Some(TokenOp {
                        op_type: TokenOpType::TransferFrom,
                        from: from.clone(),
                        to: to.clone(),
                        amount,
                    })
                };
            }
        }
    }

    #[inline(never)]
    pub fn get_last_op(&self) -> Option<TokenOp> {
        match self {
            Self::UnInit => None,
            Self::Init { last_op } => last_op.clone()
        }
    }

    #[inline(never)]
    pub fn is_uninit(&self) -> bool {
        match self {
            Self::UnInit => true,
            Self::Init { .. } => false
        }
    }
}

pub fn initialize_ghost_token_ops() {
    unsafe {
        GHOST_TOKEN_OPS.init();
    }
}

pub fn record_transfer(from: &Address, to: &Address, amount: i128) {
    unsafe {
        GHOST_TOKEN_OPS.record_transfer(from, to, amount);
    }
}

pub fn record_transfer_from(spender: &Address, from: &Address, to: &Address, amount: i128) {
    unsafe {
        GHOST_TOKEN_OPS.record_transfer_from(spender, from, to, amount);
    }
}

pub fn get_last_token_op() -> Option<TokenOp> {
    unsafe {
        GHOST_TOKEN_OPS.get_last_op()
    }
}

pub fn is_token_ops_uninit() -> bool {
    unsafe {
        GHOST_TOKEN_OPS.is_uninit()
    }
}