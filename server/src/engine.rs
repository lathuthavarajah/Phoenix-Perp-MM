use anyhow::Result;
use std::time::Duration;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::inventory::InventoryManager;
use crate::metrics::Metrics;
use crate::oracle::Oracle;
use crate::phoenix_client::SimulatedPhoenixClient;
use crate::risk::{RiskAction, RiskManager};
use crate::strategy::{multi_level, quoting};

/// Main market-making engine. Orchestrates the quote-risk-place loop.
pub struct Engine {
    config: Config,
    oracle: Oracle,
    inventory: InventoryManager,
    risk: RiskManager,
    client: SimulatedPhoenixClient,
    metrics: Metrics,
}

impl Engine {
    pub fn new(config: Config) -> Self {
        let oracle = Oracle::new(config.oracle.vol_window);
        let inventory = InventoryManager::new();
        let risk = RiskManager::new(config.risk.max_position, config.risk.max_drawdown_usd);
        let client = SimulatedPhoenixClient::new(&config.market.pair);
        let metrics = Metrics::new();

        Self {
            config,
            oracle,
            inventory,
            risk,
            client,
            metrics,
        }
    }

    /// Run the main loop.
    pub async fn run(&mut self) -> Result<()> {
        let cycle_duration = Duration::from_millis(self.config.engine.cycle_interval_ms);

        info!(
            pair = %self.config.market.pair,
            cycle_ms = self.config.engine.cycle_interval_ms,
            "engine starting"
        );

        loop {
            if let Err(e) = self.cycle().await {
                error!(error = %e, "cycle error");
            }
            self.metrics.cycles += 1;
            tokio::time::sleep(cycle_duration).await;
        }
    }

    /// Single market-making cycle.
    async fn cycle(&mut self) -> Result<()> {
        // 1. Fetch fair price from oracle
        let fair_price = match self.oracle.fetch_price().await {
            Ok(p) => {
                self.metrics.oracle_fetches += 1;
                p
            }
            Err(e) => {
                self.metrics.oracle_errors += 1;
                warn!(error = %e, "oracle fetch failed, using last price");
                match self.oracle.last_price() {
                    Some(p) => p,
                    None => return Err(e),
                }
            }
        };

        // 2. Compute volatility
        let volatility = self.oracle.volatility();

        // 3. Check risk limits
        let risk_action = self.risk.check(&self.inventory);
        match &risk_action {
            RiskAction::Continue => {}
            RiskAction::ReduceOnly => {
                self.metrics.risk_reduce_only += 1;
                warn!(position = self.inventory.position, "risk: reduce-only mode");
            }
            RiskAction::EmergencyCancel => {
                self.metrics.risk_emergency += 1;
                error!(drawdown = self.inventory.drawdown(), "risk: EMERGENCY CANCEL");
                self.client.cancel_and_place(&[]).await?;
                return Ok(());
            }
        }

        // 4. Compute inventory-skewed quotes
        let quote_params = quoting::compute_quotes(
            fair_price,
            self.inventory.position,
            self.config.risk.max_position,
            volatility,
            self.config.strategy.gamma,
            self.config.strategy.base_spread_bps,
        );

        // 5. Generate multi-level orders
        let levels = multi_level::generate_levels(
            quote_params.bid,
            quote_params.ask,
            self.config.strategy.num_levels,
            self.config.strategy.level_spacing_bps,
            self.config.strategy.base_size,
            self.config.strategy.size_decay,
        );

        // 6. Filter through risk manager
        let filtered = self
            .risk
            .filter_quotes(levels, &risk_action, self.inventory.position);

        // 7. Cancel-and-place via Phoenix client
        let order_count = filtered.len() as u64;
        self.client.cancel_and_place(&filtered).await?;
        self.metrics.orders_placed += order_count;

        // 8. Log status
        info!(
            cycle = self.metrics.cycles,
            fair_price = format!("{fair_price:.2}"),
            vol = format!("{volatility:.4}"),
            position = format!("{:.4}", self.inventory.position),
            bid = format!("{:.2}", quote_params.bid),
            ask = format!("{:.2}", quote_params.ask),
            spread_bps = format!(
                "{:.1}",
                (quote_params.ask - quote_params.bid) / fair_price * 10_000.0
            ),
            orders = order_count,
            realized_pnl = format!("{:.2}", self.inventory.realized_pnl),
            "cycle complete"
        );

        Ok(())
    }
}
