/// Seed prefix for the vault/position owner PDA
pub const VAULT_SEED: &[u8] = b"VAULT_SEED";

/// Seed for investor fee position owner
pub const INVESTOR_FEE_POS_OWNER: &[u8] = b"investor_fee_pos_owner";

/// Seed prefix for policy PDA
pub const POLICY_SEED: &[u8] = b"policy";

/// Seed prefix for progress PDA
pub const PROGRESS_SEED: &[u8] = b"progress";

/// Seed prefix for honorary position account
pub const HONORARY_POS_SEED: &[u8] = b"honorary_pos";

/// Seed for treasury ATA
pub const TREASURY_SEED: &[u8] = b"treasury";

/// Seconds in 24 hours
pub const SECONDS_PER_DAY: i64 = 86_400;

/// Basis points denominator (10000 = 100%)
pub const BPS_DENOMINATOR: u64 = 10_000;
