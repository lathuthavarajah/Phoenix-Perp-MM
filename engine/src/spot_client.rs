use anyhow::Result;
use tracing::info;

use crate::types::SpotQuote;

/// Simulated Phoenix Legacy LOB interface.
/// In production, this would use the Phoenix SDK to interact with the
/// on-chain limit order book. For the demo, we simulate order execution.
pub struct SpotClient {
    simulated_spread_bps: f64,
    simulated_fee_bps: f64,
}

impl SpotClient {
    pub fn new() -> Self {
        Self {
            simulated_spread_bps: 3.0,
            simulated_fee_bps: 1.0,
        }
    }

    pub async fn get_quote(&self, sol_price: f64) -> Result<SpotQuote> {
        let half_spread = sol_price * self.simulated_spread_bps / 10000.0 / 2.0;
        Ok(SpotQuote {
            best_bid: sol_price - half_spread,
            best_ask: sol_price + half_spread,
            spread_bps: self.simulated_spread_bps,
            bid_depth_1pct: 500.0,
            ask_depth_1pct: 500.0,
        })
    }

    pub async fn place_limit_buy(
        &self,
        size_sol: f64,
        price: f64,
    ) -> Result<String> {
        let fee = size_sol * price * self.simulated_fee_bps / 10000.0;
        info!(
            size = size_sol,
            price = price,
            fee = fee,
            "Spot: simulated limit buy"
        );
        Ok(format!("sim_spot_buy_{}", chrono_ts()))
    }

    pub async fn place_market_sell(
        &self,
        size_sol: f64,
        price: f64,
    ) -> Result<String> {
        let slippage = price * 0.001; // 10 bps slippage
        let fill_price = price - slippage;
        let fee = size_sol * fill_price * self.simulated_fee_bps / 10000.0;
        info!(
            size = size_sol,
            fill_price = fill_price,
            fee = fee,
            "Spot: simulated market sell"
        );
        Ok(format!("sim_spot_sell_{}", chrono_ts()))
    }

    pub async fn cancel_all_orders(&self) -> Result<String> {
        info!("Spot: cancelled all simulated orders");
        Ok("sim_cancel_all".to_string())
    }
}

fn chrono_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
