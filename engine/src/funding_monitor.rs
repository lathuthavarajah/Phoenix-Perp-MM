use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info};

use crate::perp_client::PerpClientTrait;
use crate::types::{EngineConfig, EngineState, FundingSignal, PerpMarketState, VenueFundingRate};

pub struct FundingMonitor {
    perp_client: Arc<dyn PerpClientTrait>,
    config: EngineConfig,
    signal_tx: broadcast::Sender<FundingSignal>,
    state: Arc<RwLock<EngineState>>,
}

impl FundingMonitor {
    pub fn new(
        perp_client: Arc<dyn PerpClientTrait>,
        config: EngineConfig,
        signal_tx: broadcast::Sender<FundingSignal>,
        state: Arc<RwLock<EngineState>>,
    ) -> Self {
        Self {
            perp_client,
            config,
            signal_tx,
            state,
        }
    }

    pub async fn run(&self) -> Result<()> {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));

        loop {
            interval.tick().await;

            match self.tick().await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!(error = %e, "Funding monitor tick failed");
                }
            }
        }
    }

    async fn tick(&self) -> Result<()> {
        let market_state = self.perp_client.get_market_state().await?;
        let position = self.perp_client.get_position().await?;
        let has_position = position.is_some();

        let signal = self.evaluate(&market_state, has_position);

        // Update shared state
        {
            let mut state = self.state.write().await;
            state.current_funding = vec![self.to_venue_rate(&market_state)];
            state.last_signal = Some(signal.clone());
            state.sol_price = market_state.index_price;
        }

        match &signal {
            FundingSignal::Enter { annualized_apy, .. } => {
                info!(apy = annualized_apy, "Signal: ENTER");
            }
            FundingSignal::Exit { annualized_apy, .. } => {
                info!(apy = annualized_apy, "Signal: EXIT");
            }
            FundingSignal::Hold => {
                debug!(
                    rate = market_state.funding_rate_8h,
                    apy = market_state.funding_rate_8h * 1095.0,
                    "Signal: HOLD"
                );
            }
        }

        let _ = self.signal_tx.send(signal);
        Ok(())
    }

    fn evaluate(&self, state: &PerpMarketState, has_position: bool) -> FundingSignal {
        let annualized_apy = state.funding_rate_8h * 1095.0;

        // Exit conditions (when position exists)
        if has_position {
            if annualized_apy < self.config.funding_exit_threshold_apy {
                return FundingSignal::Exit {
                    funding_rate_8h: state.funding_rate_8h,
                    annualized_apy,
                    reason: format!(
                        "APY {:.2}% below exit threshold {:.2}%",
                        annualized_apy * 100.0,
                        self.config.funding_exit_threshold_apy * 100.0
                    ),
                };
            }
            if state.funding_rate_8h < 0.0 {
                return FundingSignal::Exit {
                    funding_rate_8h: state.funding_rate_8h,
                    annualized_apy,
                    reason: "Funding rate flipped negative".to_string(),
                };
            }
        }

        // Entry conditions (when no position)
        if !has_position
            && annualized_apy > self.config.funding_entry_threshold_apy
            && state.premium > 0.0
        {
            return FundingSignal::Enter {
                funding_rate_8h: state.funding_rate_8h,
                annualized_apy,
                reason: format!(
                    "APY {:.2}% above entry threshold {:.2}%",
                    annualized_apy * 100.0,
                    self.config.funding_entry_threshold_apy * 100.0
                ),
            };
        }

        FundingSignal::Hold
    }

    fn to_venue_rate(&self, state: &PerpMarketState) -> VenueFundingRate {
        VenueFundingRate {
            venue: "Phoenix Perps".to_string(),
            symbol: "SOL-PERP".to_string(),
            rate_8h: state.funding_rate_8h,
            annualized_apy: state.funding_rate_8h * 1095.0,
            mark_price: state.mark_price,
            index_price: state.index_price,
            open_interest_long: state.open_interest_long,
            open_interest_short: state.open_interest_short,
            fetched_at_ts: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            is_simulated: state.is_simulated,
        }
    }
}
