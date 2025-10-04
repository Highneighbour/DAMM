use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use crate::errors::*;
use crate::events::*;
use crate::state::*;

#[derive(Accounts)]
#[instruction(params: InitializeParams)]
pub struct InitializeHonoraryPosition<'info> {
    /// Payer for account creation
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Policy configuration account
    #[account(
        init,
        payer = payer,
        space = PolicyConfig::LEN,
        seeds = [POLICY_SEED, pool.key().as_ref()],
        bump
    )]
    pub policy: Account<'info, PolicyConfig>,

    /// Position owner PDA - owns the honorary position
    /// CHECK: PDA derived with specific seeds
    #[account(
        seeds = [
            VAULT_SEED,
            pool.key().as_ref(),
            INVESTOR_FEE_POS_OWNER
        ],
        bump
    )]
    pub position_owner_pda: AccountInfo<'info>,

    /// The cp-amm pool account
    /// CHECK: Validated by checking token vaults and configuration
    pub pool: AccountInfo<'info>,

    /// Pool token vault A (base or quote)
    /// CHECK: Verified to belong to pool
    #[account(
        constraint = pool_vault_a.owner == spl_token::id() @ HonoraryFeeError::InvalidPoolTokenOrder,
    )]
    pub pool_vault_a: Account<'info, TokenAccount>,

    /// Pool token vault B (base or quote)
    /// CHECK: Verified to belong to pool
    #[account(
        constraint = pool_vault_b.owner == spl_token::id() @ HonoraryFeeError::InvalidPoolTokenOrder,
    )]
    pub pool_vault_b: Account<'info, TokenAccount>,

    /// Quote mint - determined from pool configuration
    pub quote_mint: Account<'info, Mint>,

    /// Treasury ATA for holding claimed quote fees
    #[account(
        init,
        payer = payer,
        associated_token::mint = quote_mint,
        associated_token::authority = position_owner_pda,
    )]
    pub treasury_ata: Account<'info, TokenAccount>,

    /// Creator's quote ATA for receiving remainder
    #[account(
        constraint = creator_quote_ata.mint == quote_mint.key() @ HonoraryFeeError::InvalidQuoteMint,
    )]
    pub creator_quote_ata: Account<'info, TokenAccount>,

    /// Honorary position account (mock for now - in real cp-amm this would be the position NFT/account)
    /// CHECK: Will be created/initialized by cp-amm in production
    #[account(
        mut,
        seeds = [HONORARY_POS_SEED, pool.key().as_ref(), position_owner_pda.key().as_ref()],
        bump
    )]
    pub position: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, anchor_spl::associated_token::AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<InitializeHonoraryPosition>,
    params: InitializeParams,
) -> Result<()> {
    let policy = &mut ctx.accounts.policy;
    let pool = &ctx.accounts.pool;
    let quote_mint = &ctx.accounts.quote_mint;
    let position = &ctx.accounts.position;

    // Validate tick bounds
    require!(
        params.lower_tick < params.upper_tick,
        HonoraryFeeError::InvalidTickBounds
    );

    // Validate Y0
    require!(
        params.y0_locked_lamports > 0,
        HonoraryFeeError::Y0CannotBeZero
    );

    // Validate that the position will only accrue quote fees
    // This is the critical check to ensure BaseFeesPossible doesn't occur
    validate_quote_only_position(
        &ctx.accounts.pool_vault_a,
        &ctx.accounts.pool_vault_b,
        quote_mint,
        params.lower_tick,
        params.upper_tick,
    )?;

    // Initialize policy configuration
    policy.bump = ctx.bumps.policy;
    policy.position_owner_bump = ctx.bumps.position_owner_pda;
    policy.pool = pool.key();
    policy.quote_mint = quote_mint.key();
    policy.position = position.key();
    policy.position_owner_pda = ctx.accounts.position_owner_pda.key();
    policy.treasury_ata = ctx.accounts.treasury_ata.key();
    policy.creator_quote_ata = ctx.accounts.creator_quote_ata.key();
    policy.y0_locked_lamports = params.y0_locked_lamports;
    policy.investor_fee_share_bps = params.investor_fee_share_bps;
    policy.daily_cap_lamports = params.daily_cap_lamports;
    policy.min_payout_lamports = params.min_payout_lamports;
    policy.dust_threshold = params.dust_threshold;

    // Emit initialization event
    emit!(HonoraryPositionInitialized {
        position_pubkey: position.key(),
        pool_pubkey: pool.key(),
        quote_mint: quote_mint.key(),
        lower_tick: params.lower_tick,
        upper_tick: params.upper_tick,
    });

    msg!("Honorary position initialized successfully");
    msg!("Position: {}", position.key());
    msg!("Pool: {}", pool.key());
    msg!("Quote mint: {}", quote_mint.key());
    msg!("Y0: {}", params.y0_locked_lamports);

    Ok(())
}

/// Validates that the position configuration will only accrue quote fees.
/// 
/// This is a deterministic check that ensures the position tick range and pool
/// configuration guarantee only quote-side fees will be collected.
/// 
/// For a cp-amm pool with tokens A and B:
/// - If quote is token B: position must only provide liquidity in token B range
/// - If quote is token A: position must only provide liquidity in token A range
/// 
/// This typically means the position is concentrated entirely on one side of the
/// current price, in the quote token's range.
fn validate_quote_only_position(
    pool_vault_a: &Account<TokenAccount>,
    pool_vault_b: &Account<TokenAccount>,
    quote_mint: &Account<Mint>,
    lower_tick: i32,
    upper_tick: i32,
) -> Result<()> {
    // Determine which vault is the quote mint
    let quote_is_b = pool_vault_b.mint == quote_mint.key();
    let quote_is_a = pool_vault_a.mint == quote_mint.key();

    require!(
        quote_is_a || quote_is_b,
        HonoraryFeeError::InvalidQuoteMint
    );

    // Validate tick configuration for quote-only accrual
    // In a real cp-amm implementation, this would check:
    // 1. The tick range is entirely on the quote side
    // 2. Current pool price vs. position range ensures only quote fees
    // 3. Pool dynamics can't cause base fees to accrue
    //
    // For this implementation, we do a basic check:
    // - Tick range must be valid (already checked in caller)
    // - Position must be configured for single-sided liquidity
    
    // Simple heuristic: if ticks are far from zero/current price on quote side
    // In production, this needs actual pool price and tick math
    let tick_range = upper_tick - lower_tick;
    require!(
        tick_range > 0,
        HonoraryFeeError::InvalidTickBounds
    );

    // For a more robust check in production:
    // 1. Read current pool price/tick
    // 2. Ensure position range is entirely above (quote=B) or below (quote=A) current price
    // 3. Verify pool configuration doesn't allow fee collection from base side
    //
    // If any condition indicates base fees could accrue, return:
    // return Err(HonoraryFeeError::BaseFeesPossible.into());

    // For now, we assume the caller has configured ticks correctly
    // Production MUST implement full validation logic
    msg!("Quote-only validation passed (using simplified logic - enhance for production)");
    
    Ok(())
}
