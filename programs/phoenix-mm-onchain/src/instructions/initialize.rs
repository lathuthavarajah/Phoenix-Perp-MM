use anchor_lang::prelude::*;

use crate::errors::MmError;
use crate::state::{MmConfig, MmState};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializeParams {
    pub pyth_feed_id: Pubkey,
    pub base_spread_bps: u16,
    pub gamma_scaled: u64,
    pub num_levels: u8,
    pub level_spacing_bps: u16,
    pub base_size_lots: u64,
    pub size_decay_scaled: u64,
    pub max_position_lots: i64,
    pub max_drawdown_quote_lots: i64,
    pub max_oracle_staleness_secs: u16,
    pub volatility_bps: u16,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// The Phoenix market to make on.
    /// CHECK: Validated by the operator; Phoenix CPI will reject invalid markets.
    pub phoenix_market: UncheckedAccount<'info>,

    #[account(
        init,
        payer = authority,
        space = MmConfig::SIZE,
        seeds = [b"mm_config", authority.key().as_ref(), phoenix_market.key().as_ref()],
        bump,
    )]
    pub config: Account<'info, MmConfig>,

    #[account(
        init,
        payer = authority,
        space = MmState::SIZE,
        seeds = [b"mm_state", config.key().as_ref()],
        bump,
    )]
    pub state: Account<'info, MmState>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Initialize>, params: InitializeParams) -> Result<()> {
    // Validate params
    require!(params.num_levels > 0 && params.num_levels <= 10, MmError::InvalidConfig);
    require!(params.base_spread_bps > 0, MmError::InvalidConfig);
    require!(params.gamma_scaled > 0, MmError::InvalidConfig);
    require!(params.base_size_lots > 0, MmError::InvalidConfig);
    require!(params.max_position_lots > 0, MmError::InvalidConfig);
    require!(params.max_drawdown_quote_lots > 0, MmError::InvalidConfig);
    require!(params.max_oracle_staleness_secs > 0, MmError::InvalidConfig);
    require!(
        params.size_decay_scaled > 0 && params.size_decay_scaled <= 1_000_000,
        MmError::InvalidConfig
    );

    let config = &mut ctx.accounts.config;
    config.authority = ctx.accounts.authority.key();
    config.phoenix_market = ctx.accounts.phoenix_market.key();
    config.pyth_feed_id = params.pyth_feed_id;
    config.base_spread_bps = params.base_spread_bps;
    config.gamma_scaled = params.gamma_scaled;
    config.num_levels = params.num_levels;
    config.level_spacing_bps = params.level_spacing_bps;
    config.base_size_lots = params.base_size_lots;
    config.size_decay_scaled = params.size_decay_scaled;
    config.max_position_lots = params.max_position_lots;
    config.max_drawdown_quote_lots = params.max_drawdown_quote_lots;
    config.max_oracle_staleness_secs = params.max_oracle_staleness_secs;
    config.volatility_bps = params.volatility_bps;
    config.bump = ctx.bumps.config;

    let state = &mut ctx.accounts.state;
    state.config = config.key();
    state.position_lots = 0;
    state.avg_entry_price_scaled = 0;
    state.realized_pnl_atoms = 0;
    state.peak_pnl_atoms = 0;
    state.total_volume_lots = 0;
    state.crank_count = 0;
    state.last_crank_ts = 0;
    state.bump = ctx.bumps.state;

    msg!("MM config initialized: {} levels, {} bps spread", params.num_levels, params.base_spread_bps);
    Ok(())
}
