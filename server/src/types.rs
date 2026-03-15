use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Venue {
    Phoenix,
    Hyperliquid,
    Drift,
    Binance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TradingPair {
    #[serde(rename = "SOL-PERP")]
    SolPerp,
    #[serde(rename = "BTC-PERP")]
    BtcPerp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookLevel {
    pub price: f64,
    pub size: f64,
    pub notional: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Orderbook {
    pub venue: Venue,
    pub pair: TradingPair,
    pub bids: Vec<OrderbookLevel>,
    pub asks: Vec<OrderbookLevel>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadSnapshot {
    pub venue: Venue,
    pub pair: TradingPair,
    pub bid_price: f64,
    pub ask_price: f64,
    pub mid_price: f64,
    pub spread_absolute: f64,
    pub spread_bps: f64,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingRateSnapshot {
    pub venue: Venue,
    pub pair: TradingPair,
    pub rate_hourly: f64,
    pub rate_annualized: f64,
    pub next_funding_ms: u64,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlippageEstimate {
    pub venue: Venue,
    pub pair: TradingPair,
    pub side: Side,
    pub notional_usd: f64,
    pub avg_fill_price: f64,
    pub mid_price: f64,
    pub slippage_bps: f64,
    pub depth_consumed_usd: f64,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionQualityScore {
    pub venue: Venue,
    pub pair: TradingPair,
    pub spread_score: f64,
    pub depth_score: f64,
    pub funding_score: f64,
    pub composite_score: f64,
    pub timestamp_ms: u64,
}

/// Envelope sent over WebSocket to all frontend clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum WsMessage {
    Orderbook(Orderbook),
    Funding(FundingRateSnapshot),
    Scores(Vec<ExecutionQualityScore>),
}
