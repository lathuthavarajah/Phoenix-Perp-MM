use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::types::{FundingRateSnapshot, Orderbook, TradingPair, Venue, WsMessage};

pub const BROADCAST_CAPACITY: usize = 256;

#[derive(Clone)]
pub struct AppState {
    pub tx: broadcast::Sender<WsMessage>,
    pub orderbooks: Arc<RwLock<HashMap<(Venue, TradingPair), Orderbook>>>,
    pub funding_rates: Arc<RwLock<HashMap<(Venue, TradingPair), FundingRateSnapshot>>>,
}

impl AppState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            tx,
            orderbooks: Arc::new(RwLock::new(HashMap::new())),
            funding_rates: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
