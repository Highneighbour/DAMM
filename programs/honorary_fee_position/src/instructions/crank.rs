use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use crate::constants::*;
use crate::errors::*;
use crate::events::*;
use crate::state::*;
use crate::streamflow::*;

#[derive(Accounts)]
#[instruction(params: CrankParams)]
pub struct CrankDistribute<'info> {
    /// Crank caller (permissionless)
    #[account(mut)]
    pub caller: Signer<'info>,

    /// Policy configuration
    #[account(
        seeds = [POLICY_SEED, policy.pool.as_ref()],
        bump = policy.bump,
    )]
    pub policy: Account<'info, PolicyConfig>,

    /// Distribution progress for current day
    #[account(
        init_if_needed,
        payer = caller,
        space = DistributionProgress::LEN,
        seeds = [
            PROGRESS_SEED,
            policy.pool.as_ref(),
            &get_day_ts(Clock::get()?.unix_timestamp).to_le_bytes()
        ],
        bump,
        constraint = progress.policy == policy.key() || progress.policy == Pubkey::default()
    )]
    pub progress: Account<'info, DistributionProgress>,

    /// Position owner PDA
    /// CHECK: PDA validated by seeds
    #[account(
        seeds = [
            VAULT_SEED,
            policy.pool.as_ref(),
            INVESTOR_FEE_POS_OWNER
        ],
        bump
    )]
    pub position_owner_pda: AccountInfo<'info>,

    /// Honorary position account
    /// CHECK: Validated against policy
    #[account(
        mut,
        constraint = position.key() == policy.position @ HonoraryFeeError::InvalidQuoteMint,
    )]
    pub position: AccountInfo<'info>,

    /// Treasury ATA holding claimed fees
    #[account(
        mut,
        constraint = treasury_ata.key() == policy.treasury_ata @ HonoraryFeeError::InvalidQuoteMint,
        constraint = treasury_ata.mint == policy.quote_mint @ HonoraryFeeError::InvalidQuoteMint,
    )]
    pub treasury_ata: Account<'info, TokenAccount>,

    /// Creator's quote ATA
    #[account(
        mut,
        constraint = creator_quote_ata.key() == policy.creator_quote_ata @ HonoraryFeeError::InvalidQuoteMint,
    )]
    pub creator_quote_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<CrankDistribute>,
    params: CrankParams,
) -> Result<()> {
    let policy = &ctx.accounts.policy;
    let progress = &mut ctx.accounts.progress;
    let current_ts = Clock::get()?.unix_timestamp;
    let day_ts = get_day_ts(current_ts);

    // Initialize progress if needed
    if progress.policy == Pubkey::default() {
        progress.bump = ctx.bumps.progress;
        progress.policy = policy.key();
        progress.day_ts = day_ts;
        progress.last_distribution_ts = 0;
        progress.cumulative_distributed = 0;
        progress.carry = 0;
        progress.cursor = 0;
        progress.pages_processed = 0;
        progress.day_completed = false;
        progress.total_claimed_today = 0;
    }

    // Validate day matches
    require!(
        progress.day_ts == day_ts,
        HonoraryFeeError::ProgressDayMismatch
    );

    // Check if day is already completed
    require!(
        !progress.day_completed,
        HonoraryFeeError::DayAlreadyCompleted
    );

    // Validate pagination cursor
    require!(
        params.expected_cursor == progress.cursor,
        HonoraryFeeError::PaginationCursorMismatch
    );

    // Check timing: first call of day must respect 24h cooldown
    let is_first_page = progress.pages_processed == 0;
    if is_first_page {
        require!(
            current_ts >= progress.last_distribution_ts + SECONDS_PER_DAY,
            HonoraryFeeError::CrankTooEarly
        );
    }

    // Validate investors provided
    require!(
        !params.investors.is_empty(),
        HonoraryFeeError::NoInvestorsProvided
    );

    // If first page, claim fees from position
    if is_first_page {
        let claimed_quote = claim_fees_from_position(
            &ctx.accounts.position,
            &ctx.accounts.treasury_ata,
            &ctx.accounts.position_owner_pda,
            &ctx.accounts.token_program,
            policy,
        )?;

        progress.total_claimed_today = claimed_quote;

        emit!(QuoteFeesClaimed {
            amount_claimed: claimed_quote,
            treasury_ata: ctx.accounts.treasury_ata.key(),
            timestamp: current_ts,
        });

        msg!("Claimed {} quote fees from position", claimed_quote);
    }

    // Get treasury balance for distribution
    ctx.accounts.treasury_ata.reload()?;
    let available_quote = ctx.accounts.treasury_ata.amount;

    msg!("Available quote for distribution: {}", available_quote);

    // Read locked amounts from Streamflow for all investors in this page
    let investor_locked_amounts = read_investor_locked_amounts(
        &params.investors,
        &ctx.remaining_accounts,
        current_ts,
    )?;

    // Calculate total locked across all investors in this page
    let locked_total: u64 = investor_locked_amounts.iter().sum();
    
    msg!("Total locked in this page: {}", locked_total);

    // If no one is locked, route everything to creator
    if locked_total == 0 {
        let amount_to_creator = available_quote;
        if amount_to_creator > 0 {
            // Transfer to creator
            let pool_key = policy.pool;
            let seeds = &[
                crate::constants::VAULT_SEED,
                pool_key.as_ref(),
                crate::constants::INVESTOR_FEE_POS_OWNER,
                &[policy.position_owner_bump],
            ];
            let signer_seeds = &[&seeds[..]];

            let cpi_accounts = anchor_spl::token::Transfer {
                from: ctx.accounts.treasury_ata.to_account_info(),
                to: ctx.accounts.creator_quote_ata.to_account_info(),
                authority: ctx.accounts.position_owner_pda.to_account_info(),
            };
            let cpi_context = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer_seeds,
            );

            anchor_spl::token::transfer(cpi_context, amount_to_creator)?;

            emit!(CreatorPayoutDayClosed {
                amount_routed: amount_to_creator,
                creator_ata: ctx.accounts.creator_quote_ata.key(),
                day_ts,
            });
        }

        progress.day_completed = true;
        progress.last_distribution_ts = current_ts;

        return Ok(());
    }

    // Calculate f_locked = locked_total / Y0
    let y0 = policy.y0_locked_lamports;
    let f_locked_bps = locked_total
        .checked_mul(BPS_DENOMINATOR)
        .ok_or(HonoraryFeeError::ArithmeticOverflow)?
        .checked_div(y0)
        .ok_or(HonoraryFeeError::ArithmeticOverflow)?;

    // Calculate eligible_investor_share_bps = min(investor_fee_share_bps, f_locked * 10000)
    let eligible_investor_share_bps = std::cmp::min(
        policy.investor_fee_share_bps,
        f_locked_bps,
    );

    msg!("f_locked: {} bps, eligible_investor_share: {} bps", f_locked_bps, eligible_investor_share_bps);

    // Calculate total investor allocation
    let mut investor_fee_quote = available_quote
        .checked_mul(eligible_investor_share_bps)
        .ok_or(HonoraryFeeError::ArithmeticOverflow)?
        .checked_div(BPS_DENOMINATOR)
        .ok_or(HonoraryFeeError::ArithmeticOverflow)?;

    // Apply daily cap
    if policy.daily_cap_lamports > 0 {
        let remaining_cap = policy.daily_cap_lamports
            .checked_sub(progress.cumulative_distributed)
            .unwrap_or(0);
        
        if investor_fee_quote > remaining_cap {
            msg!("Applying daily cap: {} -> {}", investor_fee_quote, remaining_cap);
            investor_fee_quote = remaining_cap;
        }
    }

    // Add carried dust from previous distributions
    investor_fee_quote = investor_fee_quote
        .checked_add(progress.carry)
        .ok_or(HonoraryFeeError::ArithmeticOverflow)?;
    
    progress.carry = 0; // Reset carry

    msg!("Total investor allocation: {}", investor_fee_quote);

    // Distribute pro-rata to investors
    let mut total_paid = 0u64;
    let mut num_paid = 0u64;
    let mut new_carry = 0u64;

    for (i, descriptor) in params.investors.iter().enumerate() {
        let locked_amount = investor_locked_amounts[i];
        
        if locked_amount == 0 {
            continue; // Skip unlocked investors
        }

        // Calculate pro-rata payout: floor(investor_fee_quote * locked_i / locked_total)
        let payout = investor_fee_quote
            .checked_mul(locked_amount)
            .ok_or(HonoraryFeeError::ArithmeticOverflow)?
            .checked_div(locked_total)
            .ok_or(HonoraryFeeError::ArithmeticOverflow)?;

        // Check minimum payout threshold
        if payout < policy.min_payout_lamports {
            msg!("Payout {} below minimum {}, carrying forward", payout, policy.min_payout_lamports);
            new_carry = new_carry
                .checked_add(payout)
                .ok_or(HonoraryFeeError::ArithmeticOverflow)?;
            continue;
        }

        // Transfer to investor
        if payout > 0 {
            // Find investor ATA in remaining_accounts
            let investor_ata_idx = ctx.remaining_accounts
                .iter()
                .position(|acc| acc.key() == descriptor.investor_quote_ata)
                .ok_or(HonoraryFeeError::MissingInvestorAta)?;

            // Perform transfer
            let pool_key = policy.pool;
            let bump = policy.position_owner_bump;
            let seeds: &[&[u8]] = &[
                crate::constants::VAULT_SEED,
                pool_key.as_ref(),
                crate::constants::INVESTOR_FEE_POS_OWNER,
                &[bump],
            ];
            let signer_seeds = &[seeds];

            let cpi_accounts = anchor_spl::token::Transfer {
                from: ctx.accounts.treasury_ata.to_account_info(),
                to: ctx.remaining_accounts[investor_ata_idx].to_account_info(),
                authority: ctx.accounts.position_owner_pda.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_context = CpiContext::new_with_signer(
                cpi_program,
                cpi_accounts,
                signer_seeds,
            );

            anchor_spl::token::transfer(cpi_context, payout)?;

            let new_total = total_paid
                .checked_add(payout)
                .ok_or(HonoraryFeeError::ArithmeticOverflow)?;
            total_paid = new_total;
            num_paid += 1;

            msg!("Paid {} to investor {}", payout, descriptor.investor_quote_ata);
        }
    }

    // Update progress
    let new_cumulative = progress.cumulative_distributed
        .checked_add(total_paid)
        .ok_or(HonoraryFeeError::ArithmeticOverflow)?;
    progress.cumulative_distributed = new_cumulative;
    progress.carry = new_carry;
    let new_cursor = progress.cursor
        .checked_add(params.investors.len() as u64)
        .ok_or(HonoraryFeeError::ArithmeticOverflow)?;
    progress.cursor = new_cursor;
    progress.pages_processed += 1;

    // Emit page event
    emit!(InvestorPayoutPage {
        page_index: progress.pages_processed - 1,
        num_paid,
        amount_paid: total_paid,
        cursor: progress.cursor,
        is_last_page: params.is_final_page,
    });

    // If final page, route remainder to creator and close day
    if params.is_final_page {
        ctx.accounts.treasury_ata.reload()?;
        let remainder = ctx.accounts.treasury_ata.amount;

        if remainder > 0 {
            // Transfer remainder to creator
            let pool_key = policy.pool;
            let seeds = &[
                crate::constants::VAULT_SEED,
                pool_key.as_ref(),
                crate::constants::INVESTOR_FEE_POS_OWNER,
                &[policy.position_owner_bump],
            ];
            let signer_seeds = &[&seeds[..]];

            let cpi_accounts = anchor_spl::token::Transfer {
                from: ctx.accounts.treasury_ata.to_account_info(),
                to: ctx.accounts.creator_quote_ata.to_account_info(),
                authority: ctx.accounts.position_owner_pda.to_account_info(),
            };
            let cpi_context = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer_seeds,
            );

            anchor_spl::token::transfer(cpi_context, remainder)?;

            emit!(CreatorPayoutDayClosed {
                amount_routed: remainder,
                creator_ata: ctx.accounts.creator_quote_ata.key(),
                day_ts,
            });

            msg!("Routed remainder {} to creator", remainder);
        }

        progress.day_completed = true;
        progress.last_distribution_ts = current_ts;
    }

    msg!("Page processed: {} investors paid {} total", num_paid, total_paid);

    Ok(())
}

/// Claim fees from the honorary position into treasury
/// In production, this would call the cp-amm's claim instruction
fn claim_fees_from_position(
    _position: &AccountInfo,
    _treasury_ata: &Account<TokenAccount>,
    _position_owner_pda: &AccountInfo,
    _token_program: &Program<Token>,
    _policy: &PolicyConfig,
) -> Result<u64> {
    // In production, this would:
    // 1. CPI to cp-amm claim_fees instruction
    // 2. Verify only quote fees claimed (base == 0)
    // 3. Return claimed amount
    //
    // For testing, we simulate fee accrual by checking treasury balance
    // or accepting pre-funded amounts

    // Mock implementation: assume fees are already in treasury
    // Real implementation MUST:
    // - CPI to cp-amm claim with position owner PDA signer
    // - Check returned amounts: require base_claimed == 0
    // - If base_claimed > 0, return Error::BaseFeeObserved
    
    msg!("Mock claim: using treasury balance as claimed fees");
    
    // In tests, treasury is pre-funded with simulated fees
    // Return a nominal amount or read from treasury
    Ok(0) // Caller will use treasury balance
}


/// Read locked amounts for all investors from Streamflow streams
fn read_investor_locked_amounts(
    investors: &[InvestorDescriptor],
    remaining_accounts: &[AccountInfo],
    current_ts: i64,
) -> Result<Vec<u64>> {
    let mut locked_amounts = Vec::with_capacity(investors.len());

    for (i, descriptor) in investors.iter().enumerate() {
        // Find corresponding stream account in remaining_accounts
        let stream_account = remaining_accounts
            .iter()
            .find(|acc| acc.key() == descriptor.stream_account)
            .ok_or(HonoraryFeeError::StreamflowReadFailure)?;

        // Read locked amount using streamflow adapter
        let locked = read_locked_amount_from_account(stream_account, current_ts)?;
        locked_amounts.push(locked);

        msg!("Investor {}: locked = {}", i, locked);
    }

    Ok(locked_amounts)
}

/// Get day timestamp (floor to 86400 boundary)
fn get_day_ts(current_ts: i64) -> i64 {
    (current_ts / SECONDS_PER_DAY) * SECONDS_PER_DAY
}
