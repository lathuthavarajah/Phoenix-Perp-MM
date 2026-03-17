//! CPI wrappers for Phoenix DEX instructions.
//!
//! Uses raw `invoke` with manually-constructed instruction data to avoid
//! pulling in Phoenix's full program dependency for CPI.

use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use borsh::BorshSerialize;

/// Phoenix DEX program ID: PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY
pub const PHOENIX_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    5, 208, 234, 79, 51, 115, 112, 19, 165, 99, 224, 147, 72, 237, 182, 244,
    89, 61, 145, 252, 118, 65, 249, 36, 124, 36, 65, 168, 66, 161, 187, 235,
]);

/// Phoenix log authority PDA: 7aDTsspkQNGKmrexAN7FLx9oxU3iPczSSvHNggyuqYkR
pub const PHOENIX_LOG_AUTHORITY: Pubkey = Pubkey::new_from_array([
    97, 168, 97, 115, 124, 201, 1, 140, 31, 126, 69, 145, 243, 168, 100, 198,
    200, 161, 77, 108, 203, 4, 205, 101, 236, 120, 68, 224, 62, 59, 217, 50,
]);

/// Instruction discriminants from Phoenix's PhoenixInstruction enum.
const CANCEL_ALL_WITH_FREE_FUNDS: u8 = 7;
const PLACE_MULTIPLE_POST_ONLY_WITH_FREE_FUNDS: u8 = 17;

/// A condensed order for PlaceMultiplePostOnlyOrders.
#[derive(BorshSerialize, Clone, Debug)]
pub struct CondensedOrder {
    pub price_in_ticks: u64,
    pub size_in_base_lots: u64,
    pub last_valid_slot: Option<u64>,
    pub last_valid_unix_timestamp_in_seconds: Option<u64>,
}

/// Behavior when a limit order in the batch fails.
#[derive(BorshSerialize, Clone, Debug)]
#[repr(u8)]
pub enum FailedMultipleLimitOrderBehavior {
    FailOnInsufficientFundsAndAmendOnCross = 0,
    #[allow(dead_code)]
    FailOnInsufficientFundsAndFailOnCross = 1,
    #[allow(dead_code)]
    SkipOnInsufficientFundsAndAmendOnCross = 2,
    #[allow(dead_code)]
    SkipOnInsufficientFundsAndFailOnCross = 3,
}

/// The MultipleOrderPacket struct matching Phoenix's Borsh layout.
#[derive(BorshSerialize, Clone, Debug)]
pub struct MultipleOrderPacket {
    pub bids: Vec<CondensedOrder>,
    pub asks: Vec<CondensedOrder>,
    pub client_order_id: Option<u128>,
    pub failed_multiple_limit_order_behavior: FailedMultipleLimitOrderBehavior,
}

/// Cancel all orders on a Phoenix market (free-funds variant, no token transfers).
///
/// Accounts needed: [phoenix_program, log_authority, market, trader(signer)]
pub fn cpi_cancel_all_orders<'info>(
    phoenix_program: &AccountInfo<'info>,
    log_authority: &AccountInfo<'info>,
    market: &AccountInfo<'info>,
    trader: &AccountInfo<'info>,
    signer_seeds: &[&[u8]],
) -> Result<()> {
    let ix = solana_program::instruction::Instruction {
        program_id: PHOENIX_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(PHOENIX_PROGRAM_ID, false),
            AccountMeta::new_readonly(log_authority.key(), false),
            AccountMeta::new(market.key(), false),
            AccountMeta::new_readonly(trader.key(), true),
        ],
        data: vec![CANCEL_ALL_WITH_FREE_FUNDS],
    };

    solana_program::program::invoke_signed(
        &ix,
        &[
            phoenix_program.to_account_info(),
            log_authority.to_account_info(),
            market.to_account_info(),
            trader.to_account_info(),
        ],
        &[signer_seeds],
    )?;

    Ok(())
}

/// Place multiple post-only orders on a Phoenix market (free-funds variant).
///
/// Accounts needed: [phoenix_program, log_authority, market, trader(signer), seat]
pub fn cpi_place_multiple_post_only<'info>(
    phoenix_program: &AccountInfo<'info>,
    log_authority: &AccountInfo<'info>,
    market: &AccountInfo<'info>,
    trader: &AccountInfo<'info>,
    seat: &AccountInfo<'info>,
    bids: Vec<CondensedOrder>,
    asks: Vec<CondensedOrder>,
    signer_seeds: &[&[u8]],
) -> Result<()> {
    let packet = MultipleOrderPacket {
        bids,
        asks,
        client_order_id: None,
        failed_multiple_limit_order_behavior:
            FailedMultipleLimitOrderBehavior::FailOnInsufficientFundsAndAmendOnCross,
    };

    let mut data = vec![PLACE_MULTIPLE_POST_ONLY_WITH_FREE_FUNDS];
    data.extend_from_slice(&packet.try_to_vec().map_err(|_| ProgramError::InvalidInstructionData)?);

    let ix = solana_program::instruction::Instruction {
        program_id: PHOENIX_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(PHOENIX_PROGRAM_ID, false),
            AccountMeta::new_readonly(log_authority.key(), false),
            AccountMeta::new(market.key(), false),
            AccountMeta::new_readonly(trader.key(), true),
            AccountMeta::new_readonly(seat.key(), false),
        ],
        data,
    };

    solana_program::program::invoke_signed(
        &ix,
        &[
            phoenix_program.to_account_info(),
            log_authority.to_account_info(),
            market.to_account_info(),
            trader.to_account_info(),
            seat.to_account_info(),
        ],
        &[signer_seeds],
    )?;

    Ok(())
}

/// Derive the Phoenix seat PDA for a trader on a market.
pub fn get_seat_address(market: &Pubkey, trader: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"seat", market.as_ref(), trader.as_ref()],
        &PHOENIX_PROGRAM_ID,
    )
}
