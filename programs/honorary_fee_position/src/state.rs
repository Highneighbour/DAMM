use anchor_lang::prelude::*;

/// Policy configuration for fee distribution
#[account]
pub struct PolicyConfig {
    /// Bump seed for PDA derivation
    pub bump: u8,
    /// Position owner PDA bump
    pub position_owner_bump: u8,
    /// The cp-amm pool this policy applies to
    pub pool: Pubkey,
    /// Quote mint of the pool
    pub quote_mint: Pubkey,
    /// The honorary position account
    pub position: Pubkey,
    /// Position owner PDA
    pub position_owner_pda: Pubkey,
    /// Treasury ATA for holding claimed fees
    pub treasury_ata: Pubkey,
    /// Creator's quote ATA for receiving remainder
    pub creator_quote_ata: Pubkey,
    /// Initial total locked amount Y0 (denominator for f_locked calculation)
    pub y0_locked_lamports: u64,
    /// Maximum investor share in basis points (0-10000)
    pub investor_fee_share_bps: u64,
    /// Optional daily cap in quote lamports (0 = no cap)
    pub daily_cap_lamports: u64,
    /// Minimum payout per investor in quote lamports
    pub min_payout_lamports: u64,
    /// Dust threshold - amounts below this are carried forward
    pub dust_threshold: u64,
}

impl PolicyConfig {
    pub const LEN: usize = 8 + // discriminator
        1 + // bump
        1 + // position_owner_bump
        32 + // pool
        32 + // quote_mint
        32 + // position
        32 + // position_owner_pda
        32 + // treasury_ata
        32 + // creator_quote_ata
        8 + // y0_locked_lamports
        8 + // investor_fee_share_bps
        8 + // daily_cap_lamports
        8 + // min_payout_lamports
        8; // dust_threshold
}

/// Progress tracking for daily distribution
#[account]
pub struct DistributionProgress {
    /// Bump seed for PDA derivation
    pub bump: u8,
    /// The policy this progress belongs to
    pub policy: Pubkey,
    /// Day timestamp (floor(now / 86400) * 86400)
    pub day_ts: i64,
    /// Last distribution timestamp
    pub last_distribution_ts: i64,
    /// Cumulative amount distributed to investors today
    pub cumulative_distributed: u64,
    /// Carried dust from previous distributions
    pub carry: u64,
    /// Current pagination cursor
    pub cursor: u64,
    /// Total pages processed today
    pub pages_processed: u64,
    /// Whether the day is completed (final page processed)
    pub day_completed: bool,
    /// Total claimed from pool today
    pub total_claimed_today: u64,
}

impl DistributionProgress {
    pub const LEN: usize = 8 + // discriminator
        1 + // bump
        32 + // policy
        8 + // day_ts
        8 + // last_distribution_ts
        8 + // cumulative_distributed
        8 + // carry
        8 + // cursor
        8 + // pages_processed
        1 + // day_completed
        8; // total_claimed_today
}

/// Parameters for initializing an honorary position
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitializeParams {
    /// Lower tick bound for the position
    pub lower_tick: i32,
    /// Upper tick bound for the position
    pub upper_tick: i32,
    /// Initial total locked amount Y0
    pub y0_locked_lamports: u64,
    /// Maximum investor share in basis points (0-10000)
    pub investor_fee_share_bps: u64,
    /// Optional daily cap in quote lamports (0 = no cap)
    pub daily_cap_lamports: u64,
    /// Minimum payout per investor in quote lamports
    pub min_payout_lamports: u64,
    /// Dust threshold
    pub dust_threshold: u64,
}

/// Investor descriptor for distribution
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InvestorDescriptor {
    /// Streamflow stream account for this investor
    pub stream_account: Pubkey,
    /// Investor's quote token ATA
    pub investor_quote_ata: Pubkey,
}

/// Parameters for crank distribution
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CrankParams {
    /// List of investors for this page
    pub investors: Vec<InvestorDescriptor>,
    /// Expected cursor position for idempotency check
    pub expected_cursor: u64,
    /// Whether this is the final page of the day
    pub is_final_page: bool,
}
