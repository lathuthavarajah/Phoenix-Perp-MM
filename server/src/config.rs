use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub market: MarketConfig,
    pub strategy: StrategyConfig,
    pub risk: RiskConfig,
    pub oracle: OracleConfig,
    pub engine: EngineConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MarketConfig {
    /// Phoenix market pubkey
    pub market_address: String,
    /// Solana RPC URL
    pub rpc_url: String,
    /// Trading pair label for logging
    pub pair: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StrategyConfig {
    /// Base spread in basis points
    pub base_spread_bps: f64,
    /// Risk aversion parameter (gamma)
    pub gamma: f64,
    /// Number of quote levels per side
    pub num_levels: u32,
    /// Spacing between levels in basis points
    pub level_spacing_bps: f64,
    /// Size of the innermost level in base units
    pub base_size: f64,
    /// Size decay factor per level (0.0-1.0)
    pub size_decay: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RiskConfig {
    /// Maximum absolute position in base units
    pub max_position: f64,
    /// Maximum drawdown in USD before emergency cancel
    pub max_drawdown_usd: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OracleConfig {
    /// Oracle poll interval in milliseconds
    pub poll_interval_ms: u64,
    /// Number of price samples for volatility
    pub vol_window: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EngineConfig {
    /// Main loop interval in milliseconds
    pub cycle_interval_ms: u64,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let contents =
            std::fs::read_to_string(path).with_context(|| format!("reading config: {path:?}"))?;
        let config: Config =
            toml::from_str(&contents).with_context(|| "parsing config.toml")?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.strategy.base_spread_bps > 0.0,
            "base_spread_bps must be positive"
        );
        anyhow::ensure!(
            self.strategy.gamma > 0.0,
            "gamma must be positive"
        );
        anyhow::ensure!(
            self.strategy.num_levels >= 1,
            "num_levels must be >= 1"
        );
        anyhow::ensure!(
            (0.0..=1.0).contains(&self.strategy.size_decay),
            "size_decay must be in [0, 1]"
        );
        anyhow::ensure!(
            self.risk.max_position > 0.0,
            "max_position must be positive"
        );
        anyhow::ensure!(
            self.risk.max_drawdown_usd > 0.0,
            "max_drawdown_usd must be positive"
        );
        anyhow::ensure!(
            self.oracle.vol_window >= 2,
            "vol_window must be >= 2"
        );
        Ok(())
    }
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Market:")?;
        writeln!(f, "  pair:           {}", self.market.pair)?;
        writeln!(f, "  market_address: {}", self.market.market_address)?;
        writeln!(f, "  rpc_url:        {}", self.market.rpc_url)?;
        writeln!(f, "Strategy:")?;
        writeln!(f, "  base_spread:    {} bps", self.strategy.base_spread_bps)?;
        writeln!(f, "  gamma:          {}", self.strategy.gamma)?;
        writeln!(f, "  levels:         {}", self.strategy.num_levels)?;
        writeln!(f, "  level_spacing:  {} bps", self.strategy.level_spacing_bps)?;
        writeln!(f, "  base_size:      {} SOL", self.strategy.base_size)?;
        writeln!(f, "  size_decay:     {}", self.strategy.size_decay)?;
        writeln!(f, "Risk:")?;
        writeln!(f, "  max_position:   {} SOL", self.risk.max_position)?;
        writeln!(f, "  max_drawdown:   ${}", self.risk.max_drawdown_usd)?;
        writeln!(f, "Oracle:")?;
        writeln!(f, "  vol_window:     {} samples", self.oracle.vol_window)?;
        writeln!(f, "  poll_interval:  {} ms", self.oracle.poll_interval_ms)?;
        writeln!(f, "Engine:")?;
        writeln!(f, "  cycle_interval: {} ms", self.engine.cycle_interval_ms)?;
        Ok(())
    }
}
