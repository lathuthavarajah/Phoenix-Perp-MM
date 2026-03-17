use anchor_lang::prelude::*;

use crate::errors::MmError;
use crate::fixed_math::{
    compute_level_prices, compute_level_sizes, compute_quotes_fixed, pyth_price_to_scaled,
    PRICE_SCALE,
};
use crate::phoenix_cpi::{
    self, cpi_cancel_all_orders, cpi_place_multiple_post_only, CondensedOrder, PHOENIX_LOG_AUTHORITY,
    PHOENIX_PROGRAM_ID,
};
use crate::state::{MmConfig, MmState};

#[derive(Accounts)]
pub struct UpdateQuotes<'info> {
    pub cranker: Signer<'info>,

    #[account(
        has_one = authority,
        seeds = [b"mm_config", config.authority.as_ref(), config.phoenix_market.as_ref()],
        bump = config.bump,
    )]
    pub config: Account<'info, MmConfig>,

    /// The authority PDA or wallet that owns the Phoenix seat.
    /// CHECK: Validated via config.authority constraint.
    pub authority: UncheckedAccount<'info>,

    #[account(
        mut,
        has_one = config,
        seeds = [b"mm_state", config.key().as_ref()],
        bump = state.bump,
    )]
    pub state: Account<'info, MmState>,

    /// The Pyth price feed account.
    /// CHECK: Validated by reading and verifying the feed data.
    #[account(address = config.pyth_feed_id)]
    pub pyth_price_feed: UncheckedAccount<'info>,

    /// The Phoenix market account.
    /// CHECK: Validated via config.phoenix_market constraint.
    #[account(mut, address = config.phoenix_market)]
    pub phoenix_market: UncheckedAccount<'info>,

    /// Phoenix program.
    /// CHECK: Validated by address constraint.
    #[account(address = PHOENIX_PROGRAM_ID)]
    pub phoenix_program: UncheckedAccount<'info>,

    /// Phoenix log authority.
    /// CHECK: Validated by address constraint.
    #[account(address = PHOENIX_LOG_AUTHORITY)]
    pub log_authority: UncheckedAccount<'info>,

    /// Phoenix seat for the trader.
    /// CHECK: Derived from market + authority; Phoenix CPI validates.
    pub seat: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<UpdateQuotes>) -> Result<()> {
    let config = &ctx.accounts.config;
    let state = &mut ctx.accounts.state;
    let clock = Clock::get()?;

    // ---- 1. Read Pyth price ----
    let price_feed_data = ctx.accounts.pyth_price_feed.try_borrow_data()?;

    // Parse Pyth push-oracle price feed account.
    // The PriceUpdateV2 account layout from pyth-solana-receiver-sdk:
    //   8 bytes discriminator
    //   1 byte write_authority_exists flag
    //   32 bytes write_authority (if flag is 1)
    //   1 byte verification_level
    //   Then the PriceFeed struct:
    //     32 bytes feed_id
    //     8 bytes price (i64)
    //     8 bytes conf (u64)
    //     4 bytes exponent (i32)
    //     8 bytes publish_time (i64)
    //     ...
    // We parse the fields we need directly to avoid heavy deserialization deps.
    let has_write_auth = price_feed_data[8];
    let offset = if has_write_auth == 1 { 8 + 1 + 32 + 1 } else { 8 + 1 + 1 };

    // Skip feed_id (32 bytes)
    let price_offset = offset + 32;
    let price = i64::from_le_bytes(
        price_feed_data[price_offset..price_offset + 8]
            .try_into()
            .map_err(|_| MmError::InvalidPrice)?,
    );
    let conf = u64::from_le_bytes(
        price_feed_data[price_offset + 8..price_offset + 16]
            .try_into()
            .map_err(|_| MmError::InvalidPrice)?,
    );
    let expo = i32::from_le_bytes(
        price_feed_data[price_offset + 16..price_offset + 20]
            .try_into()
            .map_err(|_| MmError::InvalidPrice)?,
    );
    let publish_time = i64::from_le_bytes(
        price_feed_data[price_offset + 20..price_offset + 28]
            .try_into()
            .map_err(|_| MmError::InvalidPrice)?,
    );

    // Check staleness
    let age = clock.unix_timestamp.saturating_sub(publish_time);
    require!(
        age <= config.max_oracle_staleness_secs as i64,
        MmError::StaleOracle
    );

    // Convert to PRICE_SCALE
    let fair_price = pyth_price_to_scaled(price, expo)
        .ok_or(error!(MmError::InvalidPrice))?;
    require!(fair_price > 0, MmError::InvalidPrice);

    // ---- 2. Determine volatility ----
    let volatility_bps: i128 = if config.volatility_bps > 0 {
        config.volatility_bps as i128
    } else {
        // Derive from Pyth confidence interval: vol ≈ conf / price * 10000
        let conf_scaled = pyth_price_to_scaled(conf as i64, expo).unwrap_or(0);
        if fair_price > 0 {
            (conf_scaled * 10000 / fair_price).max(100) // floor at 1% vol
        } else {
            500 // 5% default
        }
    };

    // ---- 3. Compute A-S quotes ----
    let quotes = compute_quotes_fixed(
        fair_price,
        state.position_lots,
        config.max_position_lots,
        volatility_bps,
        config.gamma_scaled as i128,
        config.base_spread_bps as i128,
    );

    // ---- 4. Risk check: drawdown ----
    // If realized PnL has dropped below peak by more than max_drawdown, emergency cancel only
    let drawdown = state.peak_pnl_atoms.saturating_sub(state.realized_pnl_atoms);
    let emergency = drawdown > config.max_drawdown_quote_lots as i128;

    // ---- 5. CPI: Cancel all existing orders ----
    // We don't use signer seeds since the cranker is the signer calling on behalf of authority
    // The authority must be a PDA of this program or the cranker must be the authority
    cpi_cancel_all_orders(
        &ctx.accounts.phoenix_program.to_account_info(),
        &ctx.accounts.log_authority.to_account_info(),
        &ctx.accounts.phoenix_market.to_account_info(),
        &ctx.accounts.authority.to_account_info(),
        &[], // No signer seeds needed if authority is direct signer
    )?;

    if emergency {
        msg!("Emergency cancel: drawdown {} exceeds max {}", drawdown, config.max_drawdown_quote_lots);
        // Just cancel, don't place new orders
        state.crank_count = state.crank_count.saturating_add(1);
        state.last_crank_ts = clock.unix_timestamp;
        return Err(error!(MmError::MaxDrawdownExceeded));
    }

    // ---- 6. Compute level prices and sizes ----
    let (bid_prices, ask_prices) = compute_level_prices(
        quotes.bid,
        quotes.ask,
        config.num_levels,
        config.level_spacing_bps as i128,
    );
    let sizes = compute_level_sizes(
        config.base_size_lots,
        config.size_decay_scaled,
        config.num_levels,
    );

    // ---- 7. Build condensed orders with reduce-only logic ----
    // Convert PRICE_SCALE prices to Phoenix tick prices.
    // For now, use raw scaled prices divided by tick size (will need market metadata).
    // Simplified: assume 1 tick = 1 unit of price in PRICE_SCALE
    let at_max_long = state.position_lots >= config.max_position_lots;
    let at_max_short = state.position_lots <= -config.max_position_lots;

    let mut bid_orders = Vec::new();
    let mut ask_orders = Vec::new();

    for i in 0..config.num_levels as usize {
        // Skip bids if at max long (reduce-only: only allow sells)
        if !at_max_long {
            let price_ticks = (bid_prices[i] / PRICE_SCALE).max(1) as u64;
            bid_orders.push(CondensedOrder {
                price_in_ticks: price_ticks,
                size_in_base_lots: sizes[i],
                last_valid_slot: None,
                last_valid_unix_timestamp_in_seconds: None,
            });
        }

        // Skip asks if at max short (reduce-only: only allow buys)
        if !at_max_short {
            let price_ticks = (ask_prices[i] / PRICE_SCALE).max(1) as u64;
            ask_orders.push(CondensedOrder {
                price_in_ticks: price_ticks,
                size_in_base_lots: sizes[i],
                last_valid_slot: None,
                last_valid_unix_timestamp_in_seconds: None,
            });
        }
    }

    // ---- 8. CPI: Place orders ----
    if !bid_orders.is_empty() || !ask_orders.is_empty() {
        // Derive seat for validation
        let (expected_seat, _) = phoenix_cpi::get_seat_address(
            &config.phoenix_market,
            &ctx.accounts.authority.key(),
        );
        require!(
            ctx.accounts.seat.key() == expected_seat,
            MmError::InvalidConfig
        );

        cpi_place_multiple_post_only(
            &ctx.accounts.phoenix_program.to_account_info(),
            &ctx.accounts.log_authority.to_account_info(),
            &ctx.accounts.phoenix_market.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.seat.to_account_info(),
            bid_orders,
            ask_orders,
            &[], // No signer seeds if authority is direct signer
        )?;
    }

    // ---- 9. Update state ----
    state.crank_count = state.crank_count.saturating_add(1);
    state.last_crank_ts = clock.unix_timestamp;

    msg!(
        "Quotes updated: bid={} ask={} pos={} crank={}",
        quotes.bid / PRICE_SCALE,
        quotes.ask / PRICE_SCALE,
        state.position_lots,
        state.crank_count,
    );

    Ok(())
}
