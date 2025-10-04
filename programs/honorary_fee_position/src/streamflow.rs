use anchor_lang::prelude::*;
use crate::errors::HonoraryFeeError;

/// Trait for reading Streamflow stream data
/// This allows swapping between real Streamflow program and mock for testing
pub trait StreamflowAdapter {
    /// Read the still-locked amount for a stream at the current timestamp
    fn read_locked_amount(
        &self,
        stream_account: &AccountInfo,
        current_ts: i64,
    ) -> Result<u64>;
}

/// Real Streamflow adapter that reads from actual Streamflow program accounts
pub struct RealStreamflowAdapter;

impl StreamflowAdapter for RealStreamflowAdapter {
    fn read_locked_amount(
        &self,
        stream_account: &AccountInfo,
        _current_ts: i64,
    ) -> Result<u64> {
        // In a real implementation, this would deserialize the Streamflow stream account
        // and calculate the still-locked amount based on the vesting schedule.
        // 
        // Streamflow stream account structure (simplified):
        // - start_time: i64
        // - end_time: i64
        // - deposited_amount: u64
        // - withdrawn_amount: u64
        // - cliff_time: i64
        // - etc.
        //
        // For now, we'll provide a basic structure check and calculation.
        // Real integration should use the actual Streamflow SDK or account layout.

        require!(
            !stream_account.data_is_empty(),
            HonoraryFeeError::StreamflowReadFailure
        );

        // Parse Streamflow account data
        // This is a placeholder - actual implementation needs Streamflow account layout
        let data = stream_account.try_borrow_data()?;
        
        // Streamflow account discriminator check (8 bytes)
        require!(
            data.len() >= 8,
            HonoraryFeeError::StreamflowReadFailure
        );

        // For production, deserialize full Streamflow stream account:
        // let stream = Stream::try_deserialize(&mut &data[..])?;
        // let locked = stream.calculate_locked_amount(current_ts)?;
        
        // Placeholder calculation - replace with actual Streamflow logic
        // This should compute: deposited_amount - vested_amount(current_ts)
        
        // For now, return 0 as a safe default
        // Real implementation MUST properly calculate locked amount
        msg!("Warning: Using placeholder Streamflow adapter - implement proper deserialization");
        
        Ok(0)
    }
}

/// Mock Streamflow adapter for testing
pub struct MockStreamflowAdapter {
    pub locked_amounts: std::collections::HashMap<Pubkey, u64>,
}

impl MockStreamflowAdapter {
    pub fn new() -> Self {
        Self {
            locked_amounts: std::collections::HashMap::new(),
        }
    }

    pub fn set_locked_amount(&mut self, stream_account: Pubkey, amount: u64) {
        self.locked_amounts.insert(stream_account, amount);
    }
}

impl StreamflowAdapter for MockStreamflowAdapter {
    fn read_locked_amount(
        &self,
        stream_account: &AccountInfo,
        _current_ts: i64,
    ) -> Result<u64> {
        Ok(*self.locked_amounts.get(&stream_account.key()).unwrap_or(&0))
    }
}

/// Helper function to read locked amount from stream account
/// This uses a simple deserialization approach for testing
pub fn read_locked_amount_from_account(
    stream_account: &AccountInfo,
    current_ts: i64,
) -> Result<u64> {
    require!(
        !stream_account.data_is_empty(),
        HonoraryFeeError::StreamflowReadFailure
    );

    let data = stream_account.try_borrow_data()?;
    
    // Check minimum size for a mock stream account
    // Format: [discriminator: 8][deposited: 8][start_ts: 8][end_ts: 8][cliff_ts: 8]
    if data.len() < 40 {
        return Ok(0); // Invalid or empty stream
    }

    // Read deposited amount (bytes 8-16)
    let deposited = u64::from_le_bytes(
        data[8..16].try_into().map_err(|_| HonoraryFeeError::StreamflowReadFailure)?
    );

    // Read start timestamp (bytes 16-24)
    let start_ts = i64::from_le_bytes(
        data[16..24].try_into().map_err(|_| HonoraryFeeError::StreamflowReadFailure)?
    );

    // Read end timestamp (bytes 24-32)
    let end_ts = i64::from_le_bytes(
        data[24..32].try_into().map_err(|_| HonoraryFeeError::StreamflowReadFailure)?
    );

    // Calculate locked amount based on linear vesting
    if current_ts < start_ts {
        // Before vesting starts - fully locked
        Ok(deposited)
    } else if current_ts >= end_ts {
        // After vesting ends - fully unlocked
        Ok(0)
    } else {
        // During vesting - calculate linear unlock
        let elapsed = (current_ts - start_ts) as u64;
        let duration = (end_ts - start_ts) as u64;
        let vested = deposited
            .checked_mul(elapsed)
            .ok_or(HonoraryFeeError::ArithmeticOverflow)?
            .checked_div(duration)
            .ok_or(HonoraryFeeError::ArithmeticOverflow)?;
        
        deposited
            .checked_sub(vested)
            .ok_or(HonoraryFeeError::ArithmeticOverflow.into())
    }
}
