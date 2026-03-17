use anchor_lang::prelude::*;

/// Market maker configuration PDA.
/// Seeds: [b"mm_config", authority, phoenix_market]
#[account]
#[derive(Debug)]
pub struct MmConfig {
    // ---- Identity ----
    /// The authority that can update or close this config.
    pub authority: Pubkey,
    /// The Phoenix market this config targets.
    pub phoenix_market: Pubkey,
    /// Pyth price feed account (push-oracle).
    pub pyth_feed_id: Pubkey,

    // ---- Strategy params ----
    /// Minimum half-spread in basis points.
    pub base_spread_bps: u16,
    /// Risk aversion parameter, scaled by 1e6 (e.g. 0.1 → 100_000).
    pub gamma_scaled: u64,
    /// Number of quote levels per side.
    pub num_levels: u8,
    /// Spacing between levels in basis points.
    pub level_spacing_bps: u16,
    /// Base order size in lots (innermost level).
    pub base_size_lots: u64,
    /// Size decay factor per level, scaled by 1e6 (e.g. 0.5 → 500_000).
    pub size_decay_scaled: u64,

    // ---- Risk params ----
    /// Maximum absolute position in lots.
    pub max_position_lots: i64,
    /// Maximum drawdown in quote lots before emergency cancel.
    pub max_drawdown_quote_lots: i64,
    /// Maximum Pyth oracle staleness in seconds.
    pub max_oracle_staleness_secs: u16,

    // ---- Volatility ----
    /// Override volatility in bps (0 = derive from Pyth confidence).
    pub volatility_bps: u16,

    /// PDA bump.
    pub bump: u8,
}

impl MmConfig {
    // 8 (discriminator) + actual field sizes
    pub const SIZE: usize = 8  // discriminator
        + 32  // authority
        + 32  // phoenix_market
        + 32  // pyth_feed_id
        + 2   // base_spread_bps
        + 8   // gamma_scaled
        + 1   // num_levels
        + 2   // level_spacing_bps
        + 8   // base_size_lots
        + 8   // size_decay_scaled
        + 8   // max_position_lots
        + 8   // max_drawdown_quote_lots
        + 2   // max_oracle_staleness_secs
        + 2   // volatility_bps
        + 1;  // bump
}

/// Market maker runtime state PDA.
/// Seeds: [b"mm_state", config]
#[account]
#[derive(Debug)]
pub struct MmState {
    /// The config PDA this state belongs to.
    pub config: Pubkey,

    // ---- Position tracking ----
    /// Current position in base lots (positive = long, negative = short).
    pub position_lots: i64,
    /// Average entry price, scaled by 1e10.
    pub avg_entry_price_scaled: i128,
    /// Realized PnL in quote atoms.
    pub realized_pnl_atoms: i128,
    /// Peak PnL watermark for drawdown calculation (quote atoms).
    pub peak_pnl_atoms: i128,

    // ---- Stats ----
    /// Total volume traded in base lots.
    pub total_volume_lots: u64,
    /// Number of times update_quotes has been cranked.
    pub crank_count: u64,
    /// Unix timestamp of last crank.
    pub last_crank_ts: i64,

    /// PDA bump.
    pub bump: u8,
}

impl MmState {
    pub const SIZE: usize = 8   // discriminator
        + 32  // config
        + 8   // position_lots
        + 16  // avg_entry_price_scaled
        + 16  // realized_pnl_atoms
        + 16  // peak_pnl_atoms
        + 8   // total_volume_lots
        + 8   // crank_count
        + 8   // last_crank_ts
        + 1;  // bump
}
