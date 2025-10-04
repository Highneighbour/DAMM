use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod streamflow;

declare_id!("HonF33Po5itioN11111111111111111111111111111");

#[program]
pub mod honorary_fee_position {
    use super::*;
    use instructions::initialize;
    use instructions::crank;

    /// Initialize an honorary fee position that accrues quote-mint fees only.
    /// 
    /// This instruction creates a program-owned PDA position on a cp-amm pool
    /// configured to accrue fees exclusively in the pool's quote mint.
    /// 
    /// # Validation
    /// - Deterministic check that position will only accrue quote fees
    /// - If base fees are possible, fails with Error::BaseFeesPossible
    /// 
    /// # Emits
    /// - HonoraryPositionInitialized event
    pub fn initialize_honorary_position(
        ctx: Context<initialize::InitializeHonoraryPosition>,
        params: state::InitializeParams,
    ) -> Result<()> {
        initialize::handler(ctx, params)
    }

    /// Permissionless crank to claim and distribute quote fees.
    /// 
    /// Can be called once per 24h window (first call), with subsequent paginated
    /// calls allowed within the same day. Distributes fees pro-rata to investors
    /// based on their still-locked Streamflow amounts.
    /// 
    /// # Validation
    /// - First call of day: requires now >= last_distribution_ts + 86400
    /// - Subsequent calls: allowed within same day for pagination
    /// - Claimed fees must be quote-only (base == 0 or fails)
    /// 
    /// # Distribution Logic
    /// - Reads still-locked amounts from Streamflow streams
    /// - Computes f_locked = locked_total / Y0
    /// - Calculates eligible_investor_share_bps with cap
    /// - Distributes pro-rata with min_payout_lamports threshold
    /// - Applies daily cap, carries dust forward
    /// - Routes remainder to creator on final page
    /// 
    /// # Emits
    /// - QuoteFeesClaimed (first page of day)
    /// - InvestorPayoutPage (each page)
    /// - CreatorPayoutDayClosed (final page)
    pub fn crank_distribute(
        ctx: Context<crank::CrankDistribute>,
        params: state::CrankParams,
    ) -> Result<()> {
        crank::handler(ctx, params)
    }
}
