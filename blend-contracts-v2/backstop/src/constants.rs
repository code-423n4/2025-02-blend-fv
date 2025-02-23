/// Fixed-point scalar for 7 decimal numbers
pub const SCALAR_7: i128 = 1_0000000;

/// Fixed-point scalar for 14 decimal numbers
pub const SCALAR_14: i128 = 1_0000000_0000000;

/// The maximum reward zone size
pub const MAX_RZ_SIZE: u32 = 50;

/// The maximum amount of active Q4W entries that a user can have against a single backstop.
pub const MAX_Q4W_SIZE: u32 = 20;

/// The time in seconds that a Q4W entry is locked for (17 days).
pub const Q4W_LOCK_TIME: u64 = 17 * 24 * 60 * 60;

/// The maximum amount of backfilled emissions that can be emitted.
/// Represents between 3-4 months worth of token emissions.
pub const MAX_BACKFILLED_EMISSIONS: i128 = 10_000_000 * SCALAR_7;
