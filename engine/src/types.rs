use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasisPosition {
    pub spot_size: f64,
    pub perp_size: f64,
    pub spot_entry_price: f64,
    pub perp_entry_price: f64,
    pub collateral_usdc: f64,
    pub is_open: bool,
    pub opened_at_ts: u64,
}

impl BasisPosition {
    pub fn delta(&self) -> f64 {
        self.spot_size + self.perp_size
    }

    pub fn notional(&self, price: f64) -> f64 {
        self.spot_size * price
    }

    pub fn unrealized_spot_pnl(&self, current_price: f64) -> f64 {
        self.spot_size * (current_price - self.spot_entry_price)
    }

    pub fn unrealized_perp_pnl(&self, current_price: f64) -> f64 {
        self.perp_size * (current_price - self.perp_entry_price)
    }

    pub fn total_unrealized_pnl(&self, current_price: f64) -> f64 {
        self.unrealized_spot_pnl(current_price) + self.unrealized_perp_pnl(current_price)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginState {
    pub collateral_usdc: f64,
    pub unrealized_pnl: f64,
    pub total_equity: f64,
    pub maintenance_margin: f64,
    pub margin_ratio: f64,
    pub liquidation_price: f64,
    pub distance_to_liq_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FundingSignal {
    Enter {
        funding_rate_8h: f64,
        annualized_apy: f64,
        reason: String,
    },
    Exit {
        funding_rate_8h: f64,
        annualized_apy: f64,
        reason: String,
    },
    Hold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueFundingRate {
    pub venue: String,
    pub symbol: String,
    pub rate_8h: f64,
    pub annualized_apy: f64,
    pub mark_price: f64,
    pub index_price: f64,
    pub open_interest_long: f64,
    pub open_interest_short: f64,
    pub fetched_at_ts: u64,
    pub is_simulated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineState {
    pub position: Option<BasisPosition>,
    pub margin: Option<MarginState>,
    pub current_funding: Vec<VenueFundingRate>,
    pub sol_price: f64,
    pub last_signal: Option<FundingSignal>,
    pub uptime_seconds: u64,
    pub alerts: Vec<String>,
    pub mode: EngineMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineMode {
    Paper,
    Devnet,
    Mainnet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpMarketState {
    pub mark_price: f64,
    pub index_price: f64,
    pub funding_rate_8h: f64,
    pub open_interest_long: f64,
    pub open_interest_short: f64,
    pub premium: f64,
    pub is_simulated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpPosition {
    pub size_sol: f64,
    pub entry_price: f64,
    pub collateral_usdc: f64,
    pub unrealized_pnl: f64,
    pub liquidation_price: f64,
    pub margin_ratio: f64,
    pub is_simulated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpOrderResult {
    pub fill_price: f64,
    pub fee_usdc: f64,
    pub simulated: bool,
}

#[derive(Debug, Clone)]
pub struct SpotQuote {
    pub best_bid: f64,
    pub best_ask: f64,
    pub spread_bps: f64,
    pub bid_depth_1pct: f64,
    pub ask_depth_1pct: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EngineConfig {
    pub rpc_url: String,
    pub use_devnet: bool,
    pub wallet_keypair_path: String,
    pub funding_entry_threshold_apy: f64,
    pub funding_exit_threshold_apy: f64,
    pub position_size_usdc: f64,
    pub max_leverage: f64,
    pub rebalance_delta_threshold: f64,
    pub margin_warning_ratio: f64,
    pub emergency_close_ratio: f64,
    pub engine_http_port: u16,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            use_devnet: true,
            wallet_keypair_path: "~/.config/solana/id.json".to_string(),
            funding_entry_threshold_apy: 0.15,
            funding_exit_threshold_apy: 0.02,
            position_size_usdc: 500.0,
            max_leverage: 3.0,
            rebalance_delta_threshold: 0.01,
            margin_warning_ratio: 0.25,
            emergency_close_ratio: 0.12,
            engine_http_port: 8080,
        }
    }
}
