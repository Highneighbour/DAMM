use anchor_lang::prelude::*;

#[error_code]
pub enum HonoraryFeeError {
    #[msg("Base fees are possible with this position configuration - only quote fees allowed")]
    BaseFeesPossible,

    #[msg("Base fees observed during claim - position must only accrue quote fees")]
    BaseFeeObserved,

    #[msg("Invalid quote mint - position quote mint does not match pool quote mint")]
    InvalidQuoteMint,

    #[msg("Crank called too early - must wait 24h since last distribution")]
    CrankTooEarly,

    #[msg("Pagination cursor mismatch - provided cursor does not match expected state")]
    PaginationCursorMismatch,

    #[msg("Minimum payout not reached - amount below min_payout_lamports threshold")]
    MinPayoutNotReached,

    #[msg("Daily cap reached - cannot distribute more to investors today")]
    DailyCapReached,

    #[msg("Streamflow read failure - unable to read locked amounts for investor")]
    StreamflowReadFailure,

    #[msg("Missing investor ATA - investor associated token account not found")]
    MissingInvestorAta,

    #[msg("Arithmetic overflow in calculations")]
    ArithmeticOverflow,

    #[msg("Invalid pool token order - unable to determine quote mint")]
    InvalidPoolTokenOrder,

    #[msg("Invalid tick bounds - lower tick must be less than upper tick")]
    InvalidTickBounds,

    #[msg("Y0 (initial locked amount) cannot be zero")]
    Y0CannotBeZero,

    #[msg("No investors provided for this page")]
    NoInvestorsProvided,

    #[msg("Progress day mismatch - progress PDA is for a different day")]
    ProgressDayMismatch,

    #[msg("Invalid page index - page must be sequential or resuming")]
    InvalidPageIndex,

    #[msg("Day already completed - cannot crank after final page processed")]
    DayAlreadyCompleted,
}
