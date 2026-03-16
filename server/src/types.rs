use serde::{Deserialize, Serialize};

/// Side of an order or fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Bid,
    Ask,
}

/// A single quote level to place on the book.
#[derive(Debug, Clone)]
pub struct QuoteLevel {
    pub side: Side,
    pub price: f64,
    pub size: f64,
    pub level: u32,
}

/// A fill event from the exchange.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Fill {
    pub side: Side,
    pub price: f64,
    pub size: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// An orderbook level from Phoenix.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BookLevel {
    pub price: f64,
    pub size: f64,
}

/// Snapshot of the current orderbook.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OrderbookSnapshot {
    pub bids: Vec<BookLevel>,
    pub asks: Vec<BookLevel>,
}

#[allow(dead_code)]
impl OrderbookSnapshot {
    /// Best bid price, if any.
    pub fn best_bid(&self) -> Option<f64> {
        self.bids.first().map(|l| l.price)
    }

    /// Best ask price, if any.
    pub fn best_ask(&self) -> Option<f64> {
        self.asks.first().map(|l| l.price)
    }

    /// Mid price from best bid/ask.
    pub fn mid_price(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(b), Some(a)) => Some((b + a) / 2.0),
            _ => None,
        }
    }
}
