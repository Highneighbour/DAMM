use anchor_lang::prelude::*;

/// Emitted when an honorary position is successfully initialized
#[event]
pub struct HonoraryPositionInitialized {
    /// The public key of the honorary position account
    pub position_pubkey: Pubkey,
    /// The public key of the cp-amm pool
    pub pool_pubkey: Pubkey,
    /// The quote mint that will accrue fees
    pub quote_mint: Pubkey,
    /// The lower tick bound
    pub lower_tick: i32,
    /// The upper tick bound
    pub upper_tick: i32,
}

/// Emitted when quote fees are claimed from the honorary position
#[event]
pub struct QuoteFeesClaimed {
    /// Amount of quote fees claimed
    pub amount_claimed: u64,
    /// Treasury ATA that received the fees
    pub treasury_ata: Pubkey,
    /// Timestamp of the claim
    pub timestamp: i64,
}

/// Emitted for each page of investor payouts processed
#[event]
pub struct InvestorPayoutPage {
    /// Page index (0-based)
    pub page_index: u64,
    /// Number of investors paid in this page
    pub num_paid: u64,
    /// Total amount paid to investors in this page
    pub amount_paid: u64,
    /// Pagination cursor for next page (if not last)
    pub cursor: u64,
    /// Whether this is the last page of the day
    pub is_last_page: bool,
}

/// Emitted when a day's distribution is completed and remainder sent to creator
#[event]
pub struct CreatorPayoutDayClosed {
    /// Amount routed to creator
    pub amount_routed: u64,
    /// Creator's quote ATA
    pub creator_ata: Pubkey,
    /// Day timestamp
    pub day_ts: i64,
}
