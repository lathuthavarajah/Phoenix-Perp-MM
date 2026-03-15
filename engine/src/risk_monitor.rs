use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

use crate::executor::Executor;
use crate::oracle_client::OracleClient;
use crate::perp_client::PerpClientTrait;
use crate::types::{BasisPosition, EngineConfig, EngineState, MarginState, PerpMarketState};

pub struct RiskMonitor {
    perp_client: Arc<dyn PerpClientTrait>,
    oracle: Arc<OracleClient>,
    executor: Arc<Executor>,
    config: EngineConfig,
    state: Arc<RwLock<EngineState>>,
    _alerts_tx: broadcast::Sender<String>,
}

impl RiskMonitor {
    pub fn new(
        perp_client: Arc<dyn PerpClientTrait>,
        oracle: Arc<OracleClient>,
        executor: Arc<Executor>,
        config: EngineConfig,
        state: Arc<RwLock<EngineState>>,
        alerts_tx: broadcast::Sender<String>,
    ) -> Self {
        Self {
            perp_client,
            oracle,
            executor,
            config,
            state,
            _alerts_tx: alerts_tx,
        }
    }

    pub async fn run(&self) -> Result<()> {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

        loop {
            interval.tick().await;
            if let Err(e) = self.tick().await {
                tracing::error!(error = %e, "Risk monitor tick failed");
            }
        }
    }

    async fn tick(&self) -> Result<()> {
        let position = {
            let state = self.state.read().await;
            state.position.clone()
        };

        let position = match position {
            Some(p) if p.is_open => p,
            _ => return Ok(()),
        };

        let sol_price = self.oracle.get_sol_price().await?;
        let market_state = self.perp_client.get_market_state().await?;
        let margin = self.compute_margin_state(&position, &market_state, sol_price);

        // Update shared state
        {
            let mut state = self.state.write().await;
            state.margin = Some(margin.clone());
        }

        // Check margin levels
        if margin.margin_ratio < self.config.emergency_close_ratio {
            warn!(
                margin_ratio = margin.margin_ratio,
                "EMERGENCY CLOSE: margin below critical threshold"
            );

            let alert = format!(
                "EMERGENCY_CLOSE: margin_ratio={:.4}, liq_price=${:.2}",
                margin.margin_ratio, margin.liquidation_price
            );

            {
                let mut state = self.state.write().await;
                state.alerts.push(alert);
            }

            // Execute emergency close
            if let Err(e) = self.executor.close_basis_position(&position).await {
                warn!(error = %e, "Emergency close failed");
            } else {
                info!("Emergency close executed");
                let mut state = self.state.write().await;
                state.position = None;
                state.margin = None;
            }
        } else if margin.margin_ratio < self.config.margin_warning_ratio {
            let alert = format!(
                "MARGIN_WARNING: ratio={:.4}, distance_to_liq={:.2}%",
                margin.margin_ratio,
                margin.distance_to_liq_pct * 100.0
            );

            let mut state = self.state.write().await;
            // Only add warning if the last alert is different
            if state.alerts.last().map(|a| a.as_str()) != Some(&alert) {
                state.alerts.push(alert);
            }
        }

        Ok(())
    }

    pub fn compute_margin_state(
        &self,
        position: &BasisPosition,
        perp_state: &PerpMarketState,
        sol_price: f64,
    ) -> MarginState {
        let unrealized_pnl = position.perp_size * (sol_price - position.perp_entry_price);
        let total_equity = position.collateral_usdc + unrealized_pnl;
        let abs_notional = position.perp_size.abs() * perp_state.mark_price;
        let margin_ratio = if abs_notional > 0.0 {
            total_equity / abs_notional
        } else {
            1.0
        };

        let maintenance_margin = abs_notional * 0.05;

        // Liquidation price for short: price rises until equity = maintenance_margin
        let liq_price = if position.perp_size.abs() > 0.0 {
            position.perp_entry_price
                + (total_equity / position.perp_size.abs())
                - (maintenance_margin / position.perp_size.abs())
        } else {
            0.0
        };

        let distance_to_liq_pct = if sol_price > 0.0 {
            (liq_price - sol_price) / sol_price
        } else {
            0.0
        };

        MarginState {
            collateral_usdc: position.collateral_usdc,
            unrealized_pnl,
            total_equity,
            maintenance_margin,
            margin_ratio,
            liquidation_price: liq_price,
            distance_to_liq_pct,
        }
    }
}
