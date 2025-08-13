use soroban_fixed_point_math::FixedPoint;

/////////////
pub(crate) fn certora_convert_to_tokens(pool_shares: i64, pool_tokens: i64, shares: i64) -> i64 {
    if pool_shares == 0 {
        return shares;
    }
    // shares * pool_shares / pool_tokens //@audit-issue should be shares * pool_tokens / pool_shares
    shares
        .fixed_mul_floor(pool_tokens, pool_shares)
        .unwrap()
}

pub(crate) fn certora_convert_to_shares(pool_shares: i64, pool_tokens: i64, tokens: i64) -> i64 {
    if pool_shares == 0 {
        return tokens;
    }
    // tokens * pool_tokens / pool_shares //@audit-issue should be tokens * pool_shares / pool_tokens
     tokens
        .fixed_mul_floor(pool_shares, pool_tokens)
        .unwrap()
}

/////////////
