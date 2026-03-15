use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

use crate::executor::Executor;
use crate::oracle_client::OracleClient;
use crate::types::{BasisPosition, EngineConfig, EngineState, FundingSignal};

pub struct HedgeEngine {
    oracle: Arc<OracleClient>,
    executor: Arc<Executor>,
    config: EngineConfig,
    state: Arc<RwLock<EngineState>>,
    signal_rx: broadcast::Receiver<FundingSignal>,
}

impl HedgeEngine {
    pub fn new(
        oracle: Arc<OracleClient>,
        executor: Arc<Executor>,
        config: EngineConfig,
        state: Arc<RwLock<EngineState>>,
        signal_rx: broadcast::Receiver<FundingSignal>,
    ) -> Self {
        Self {
            oracle,
            executor,
            config,
            state,
            signal_rx,
        }
    }

    pub async fn run(mut self) -> Result<()> {
        loop {
            match self.signal_rx.recv().await {
                Ok(signal) => {
                    if let Err(e) = self.handle_signal(signal).await {
                        warn!(error = %e, "Failed to handle signal");
                        let mut state = self.state.write().await;
                        state.alerts.push(format!("ENTRY_FAILED: {}", e));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(skipped = n, "Hedge engine lagged, skipping signals");
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!("Signal channel closed, stopping hedge engine");
                    break;
                }
            }
        }
        Ok(())
    }

    async fn handle_signal(&self, signal: FundingSignal) -> Result<()> {
        match signal {
            FundingSignal::Enter { annualized_apy, .. } => {
                let has_position = self.state.read().await.position.is_some();
                if has_position {
                    return Ok(());
                }

                let sol_price = self.oracle.get_sol_price().await?;
                let (spot_size, perp_size, collateral) =
                    self.compute_entry_sizes(sol_price);

                info!(
                    spot = spot_size,
                    perp = perp_size,
                    collateral = collateral,
                    apy = annualized_apy,
                    "Opening basis position"
                );

                let position = self
                    .executor
                    .open_basis_position(spot_size, perp_size, collateral, sol_price)
                    .await?;

                let mut state = self.state.write().await;
                state.position = Some(position);
            }
            FundingSignal::Exit { reason, .. } => {
                let position = self.state.read().await.position.clone();
                if let Some(pos) = position {
                    info!(reason = reason, "Closing basis position");
                    self.executor.close_basis_position(&pos).await?;
                    let mut state = self.state.write().await;
                    state.position = None;
                    state.margin = None;
                }
            }
            FundingSignal::Hold => {}
        }
        Ok(())
    }

    pub fn compute_entry_sizes(&self, sol_price: f64) -> (f64, f64, f64) {
        let spot_size_sol = self.config.position_size_usdc / sol_price;
        let perp_size_sol = -spot_size_sol;
        let collateral_usdc =
            spot_size_sol * sol_price / self.config.max_leverage;
        (spot_size_sol, perp_size_sol, collateral_usdc)
    }
}
