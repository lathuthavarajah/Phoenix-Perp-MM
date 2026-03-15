use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn};

use crate::perp_client::PerpClientTrait;
use crate::spot_client::SpotClient;
use crate::types::BasisPosition;

pub struct Executor {
    spot_client: Arc<SpotClient>,
    perp_client: Arc<dyn PerpClientTrait>,
}

impl Executor {
    pub fn new(
        spot_client: Arc<SpotClient>,
        perp_client: Arc<dyn PerpClientTrait>,
    ) -> Self {
        Self {
            spot_client,
            perp_client,
        }
    }

    pub async fn open_basis_position(
        &self,
        spot_size: f64,
        perp_size: f64,
        collateral: f64,
        sol_price: f64,
    ) -> Result<BasisPosition> {
        // Submit both legs concurrently
        let spot_fut = self.spot_client.place_limit_buy(spot_size, sol_price);
        let perp_fut = self
            .perp_client
            .open_short(perp_size.abs(), collateral);

        let (spot_result, perp_result) = tokio::join!(spot_fut, perp_fut);

        match (&spot_result, &perp_result) {
            (Ok(_), Ok(perp_order)) => {
                info!("Both legs filled successfully");
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                Ok(BasisPosition {
                    spot_size,
                    perp_size,
                    spot_entry_price: sol_price,
                    perp_entry_price: perp_order.fill_price,
                    collateral_usdc: collateral,
                    is_open: true,
                    opened_at_ts: now,
                })
            }
            (Err(e), Ok(_)) => {
                warn!(error = %e, "Spot leg failed, closing perp");
                let _ = self.perp_client.close_position().await;
                anyhow::bail!("Spot leg failed: {}", e)
            }
            (Ok(_), Err(e)) => {
                warn!(error = %e, "Perp leg failed, cancelling spot");
                let _ = self.spot_client.cancel_all_orders().await;
                anyhow::bail!("Perp leg failed: {}", e)
            }
            (Err(e1), Err(e2)) => {
                anyhow::bail!("Both legs failed: spot={}, perp={}", e1, e2)
            }
        }
    }

    pub async fn close_basis_position(&self, position: &BasisPosition) -> Result<()> {
        let spot_fut = self
            .spot_client
            .place_market_sell(position.spot_size, position.spot_entry_price);
        let perp_fut = self.perp_client.close_position();

        let (spot_result, perp_result) = tokio::join!(spot_fut, perp_fut);

        if let Err(e) = &spot_result {
            warn!(error = %e, "Failed to close spot leg");
        }
        if let Err(e) = &perp_result {
            warn!(error = %e, "Failed to close perp leg");
        }

        spot_result?;
        perp_result?;

        info!("Basis position closed");
        Ok(())
    }
}
