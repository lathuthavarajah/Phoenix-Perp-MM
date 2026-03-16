use anyhow::Result;
use std::collections::VecDeque;
use tracing::{debug, warn};

use crate::error::MakerError;

/// Coinbase spot price response.
#[derive(serde::Deserialize)]
struct CoinbaseResponse {
    data: CoinbaseData,
}

#[derive(serde::Deserialize)]
struct CoinbaseData {
    amount: String,
}

pub struct Oracle {
    client: reqwest::Client,
    price_history: VecDeque<f64>,
    vol_window: usize,
    last_price: Option<f64>,
}

impl Oracle {
    pub fn new(vol_window: usize) -> Self {
        Self {
            client: reqwest::Client::new(),
            price_history: VecDeque::with_capacity(vol_window + 1),
            vol_window,
            last_price: None,
        }
    }

    /// Fetch SOL-USD spot price from Coinbase.
    pub async fn fetch_price(&mut self) -> Result<f64> {
        let resp: CoinbaseResponse = self
            .client
            .get("https://api.coinbase.com/v2/prices/SOL-USD/spot")
            .send()
            .await
            .map_err(|e| MakerError::Oracle(format!("request failed: {e}")))?
            .json()
            .await
            .map_err(|e| MakerError::Oracle(format!("parse failed: {e}")))?;

        let price: f64 = resp
            .data
            .amount
            .parse()
            .map_err(|e| MakerError::Oracle(format!("invalid price: {e}")))?;

        debug!(price, "fetched oracle price");

        // Update rolling history
        self.price_history.push_back(price);
        if self.price_history.len() > self.vol_window {
            self.price_history.pop_front();
        }
        self.last_price = Some(price);

        Ok(price)
    }

    /// Get last fetched price without a network call.
    pub fn last_price(&self) -> Option<f64> {
        self.last_price
    }

    /// Compute realized volatility from log-returns of the price history.
    /// Returns annualized vol as a decimal (e.g., 0.80 = 80%).
    /// When fewer than `min_samples` prices are available, returns a high default.
    pub fn volatility(&self) -> f64 {
        let min_samples = 5;
        let prices: Vec<f64> = self.price_history.iter().copied().collect();

        if prices.len() < min_samples {
            warn!(
                samples = prices.len(),
                required = min_samples,
                "insufficient samples, using default high volatility"
            );
            return 0.80; // 80% annualized default
        }

        let log_returns: Vec<f64> = prices
            .windows(2)
            .map(|w| (w[1] / w[0]).ln())
            .collect();

        let n = log_returns.len() as f64;
        let mean = log_returns.iter().sum::<f64>() / n;
        let variance = log_returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let std_dev = variance.sqrt();

        // Annualize: assume each sample is ~2s apart, ~15,768,000 two-second intervals/year
        // But for practical purposes, scale by sqrt(samples_per_day)
        // With 2s intervals: 43,200 samples/day, 365 days
        let samples_per_year: f64 = 43_200.0 * 365.0;
        let annualized = std_dev * samples_per_year.sqrt();

        debug!(
            samples = prices.len(),
            raw_std = std_dev,
            annualized,
            "computed volatility"
        );

        annualized.max(0.01) // Floor at 1%
    }

    /// Number of price samples currently stored.
    #[allow(dead_code)]
    pub fn sample_count(&self) -> usize {
        self.price_history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volatility_insufficient_samples() {
        let mut oracle = Oracle::new(20);
        // Add only 3 samples
        oracle.price_history.push_back(100.0);
        oracle.price_history.push_back(101.0);
        oracle.price_history.push_back(102.0);

        let vol = oracle.volatility();
        assert!((vol - 0.80).abs() < 1e-10, "should return default 80% vol");
    }

    #[test]
    fn test_volatility_stable_prices() {
        let mut oracle = Oracle::new(20);
        // Stable prices → low vol
        for _ in 0..10 {
            oracle.price_history.push_back(100.0);
        }
        let vol = oracle.volatility();
        assert!(vol <= 0.02, "stable prices should have near-zero vol, got {vol}");
    }

    #[test]
    fn test_volatility_volatile_prices() {
        let mut oracle = Oracle::new(20);
        // Alternating prices → high vol
        for i in 0..10 {
            let price = if i % 2 == 0 { 100.0 } else { 105.0 };
            oracle.price_history.push_back(price);
        }
        let vol = oracle.volatility();
        assert!(vol > 1.0, "volatile prices should have high vol, got {vol}");
    }
}
