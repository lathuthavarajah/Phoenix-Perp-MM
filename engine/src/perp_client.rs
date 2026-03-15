use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use crate::oracle_client::OracleClient;
use crate::types::{PerpMarketState, PerpOrderResult, PerpPosition};

/// Mock Phoenix Perpetuals client.
/// Simulates perp market state, position management, and funding rates.
/// Designed as a trait so the real SDK can be swapped in later.
#[async_trait::async_trait]
pub trait PerpClientTrait: Send + Sync {
    async fn get_market_state(&self) -> Result<PerpMarketState>;
    async fn get_position(&self) -> Result<Option<PerpPosition>>;
    async fn open_short(&self, size_sol: f64, collateral_usdc: f64) -> Result<PerpOrderResult>;
    async fn close_position(&self) -> Result<PerpOrderResult>;
    async fn add_collateral(&self, amount_usdc: f64) -> Result<()>;
}

pub struct MockPerpClient {
    position: Arc<Mutex<Option<PerpPosition>>>,
    oracle: Arc<OracleClient>,
    simulated_premium: Arc<Mutex<f64>>,
    simulated_slippage_bps: f64,
    simulated_fee_bps: f64,
}

impl MockPerpClient {
    pub fn new(oracle: Arc<OracleClient>) -> Self {
        Self {
            position: Arc::new(Mutex::new(None)),
            oracle,
            simulated_premium: Arc::new(Mutex::new(0.001)),
            simulated_slippage_bps: 5.0,
            simulated_fee_bps: 5.0,
        }
    }

    fn compute_funding_rate(mark_price: f64, index_price: f64) -> f64 {
        let premium = (mark_price - index_price) / index_price;
        let raw_rate = premium * 0.1;
        raw_rate.clamp(-0.003, 0.003)
    }

    fn compute_liquidation_price(position: &PerpPosition) -> f64 {
        let maintenance_margin_rate = 0.05_f64;
        let notional = position.size_sol.abs() * position.entry_price;
        let equity = position.collateral_usdc + position.unrealized_pnl;
        position.entry_price * (1.0 + (equity / notional) - maintenance_margin_rate)
    }

    fn compute_margin_ratio(position: &PerpPosition, mark_price: f64) -> f64 {
        let notional = position.size_sol.abs() * mark_price;
        if notional == 0.0 {
            return 1.0;
        }
        let equity = position.collateral_usdc + position.unrealized_pnl;
        equity / notional
    }

    async fn update_premium(&self) {
        let mut premium = self.simulated_premium.lock().await;
        // Mean-revert toward 0.001 with noise
        let noise = (rand::random::<f64>() - 0.5) * 0.0004;
        *premium = (*premium * 0.95 + 0.001 * 0.05) + noise;
        *premium = premium.clamp(-0.005, 0.005);
    }
}

#[async_trait::async_trait]
impl PerpClientTrait for MockPerpClient {
    async fn get_market_state(&self) -> Result<PerpMarketState> {
        self.update_premium().await;

        let index_price = self.oracle.get_sol_price().await?;
        let premium = *self.simulated_premium.lock().await;
        let mark_price = index_price * (1.0 + premium);
        let funding_rate_8h = Self::compute_funding_rate(mark_price, index_price);

        // Update position PnL if one exists
        {
            let mut pos = self.position.lock().await;
            if let Some(ref mut p) = *pos {
                p.unrealized_pnl = p.size_sol * (mark_price - p.entry_price);
                p.margin_ratio = Self::compute_margin_ratio(p, mark_price);
                p.liquidation_price = Self::compute_liquidation_price(p);
            }
        }

        Ok(PerpMarketState {
            mark_price,
            index_price,
            funding_rate_8h,
            open_interest_long: 1_250_000.0 + rand::random::<f64>() * 100_000.0,
            open_interest_short: 1_180_000.0 + rand::random::<f64>() * 100_000.0,
            premium,
            is_simulated: true,
        })
    }

    async fn get_position(&self) -> Result<Option<PerpPosition>> {
        Ok(self.position.lock().await.clone())
    }

    async fn open_short(&self, size_sol: f64, collateral_usdc: f64) -> Result<PerpOrderResult> {
        let index_price = self.oracle.get_sol_price().await?;
        let slippage = index_price * self.simulated_slippage_bps / 10000.0;
        let fill_price = index_price - slippage;
        let fee = size_sol * fill_price * self.simulated_fee_bps / 10000.0;

        let position = PerpPosition {
            size_sol: -size_sol.abs(),
            entry_price: fill_price,
            collateral_usdc,
            unrealized_pnl: 0.0,
            liquidation_price: fill_price * (1.0 + (collateral_usdc / (size_sol * fill_price)) - 0.05),
            margin_ratio: collateral_usdc / (size_sol * fill_price),
            is_simulated: true,
        };

        *self.position.lock().await = Some(position);

        info!(
            size = -size_sol,
            fill_price = fill_price,
            fee = fee,
            "Perp: simulated short opened"
        );

        Ok(PerpOrderResult {
            fill_price,
            fee_usdc: fee,
            simulated: true,
        })
    }

    async fn close_position(&self) -> Result<PerpOrderResult> {
        let pos = self.position.lock().await.clone();
        let current_price = self.oracle.get_sol_price().await?;

        let fill_price = if let Some(ref p) = pos {
            let slippage = current_price * self.simulated_slippage_bps / 10000.0;
            let fee = p.size_sol.abs() * current_price * self.simulated_fee_bps / 10000.0;

            info!(
                size = p.size_sol,
                fill_price = current_price + slippage,
                pnl = p.unrealized_pnl,
                "Perp: simulated position closed"
            );

            *self.position.lock().await = None;

            Ok(PerpOrderResult {
                fill_price: current_price + slippage,
                fee_usdc: fee,
                simulated: true,
            })
        } else {
            anyhow::bail!("No position to close");
        };

        fill_price
    }

    async fn add_collateral(&self, amount_usdc: f64) -> Result<()> {
        let mut pos = self.position.lock().await;
        if let Some(ref mut p) = *pos {
            p.collateral_usdc += amount_usdc;
            info!(
                added = amount_usdc,
                total = p.collateral_usdc,
                "Perp: collateral added"
            );
        }
        Ok(())
    }
}
