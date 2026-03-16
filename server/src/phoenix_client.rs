use anyhow::Result;
use tracing::{debug, info};

use crate::types::{Fill, OrderbookSnapshot, QuoteLevel, Side};
#[cfg(feature = "live")]
use crate::types::BookLevel;

// ---------------------------------------------------------------------------
// Live Phoenix SDK client (behind `live` feature flag)
// ---------------------------------------------------------------------------
#[cfg(feature = "live")]
pub mod live {
    use super::*;
    use crate::error::MakerError;
    use phoenix_sdk::sdk_client::SDKClient;
    use solana_client::rpc_client::RpcClient;
    use solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
    };
    use std::str::FromStr;

    pub struct LivePhoenixClient {
        sdk: SDKClient,
        market: Pubkey,
        trader: Keypair,
    }

    impl LivePhoenixClient {
        pub async fn new(rpc_url: &str, market_address: &str, trader: Keypair) -> Result<Self> {
            let market = Pubkey::from_str(market_address)
                .map_err(|e| MakerError::Phoenix(format!("invalid market address: {e}")))?;

            info!(
                market = %market,
                trader = %trader.pubkey(),
                "connecting to Phoenix market"
            );

            let client = RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            );
            let mut sdk = SDKClient::new(&trader, &client)
                .await
                .map_err(|e| MakerError::Phoenix(format!("SDK init failed: {e}")))?;

            sdk.add_market(market_address)
                .await
                .map_err(|e| MakerError::Phoenix(format!("add market failed: {e}")))?;

            info!("Phoenix client connected");

            Ok(Self { sdk, market, trader })
        }
    }

    impl PhoenixExchange for LivePhoenixClient {
        async fn get_ladder(&self, levels: usize) -> Result<OrderbookSnapshot> {
            let ladder = self
                .sdk
                .get_market_ladder(&self.market, levels)
                .map_err(|e| MakerError::Phoenix(format!("get ladder failed: {e}")))?;

            let bids = ladder.bids.iter().map(|&(price, size)| BookLevel { price, size }).collect();
            let asks = ladder.asks.iter().map(|&(price, size)| BookLevel { price, size }).collect();

            Ok(OrderbookSnapshot { bids, asks })
        }

        async fn cancel_and_place(&self, quotes: &[QuoteLevel]) -> Result<Vec<Fill>> {
            let cancel_sig = self
                .sdk
                .send_cancel_all(&self.market, &self.trader)
                .await
                .map_err(|e| MakerError::Phoenix(format!("cancel failed: {e}")))?;

            debug!(sig = %cancel_sig, "cancelled existing orders");

            for q in quotes {
                let market_side = match q.side {
                    Side::Bid => phoenix_sdk::sdk_client::MarketSide::Bid,
                    Side::Ask => phoenix_sdk::sdk_client::MarketSide::Ask,
                };

                self.sdk
                    .send_limit_order(
                        &self.market,
                        &self.trader,
                        q.price,
                        phoenix_sdk::order_packet_template::SelfTradeBehavior::CancelProvide,
                        None,
                        q.size,
                        phoenix_sdk::order_packet_template::OrderType::Limit,
                        market_side,
                        false,
                        None,
                        None,
                    )
                    .await
                    .map_err(|e| MakerError::Phoenix(format!("place order failed: {e}")))?;
            }

            debug!(orders = quotes.len(), "placed new orders");
            Ok(Vec::new()) // Fill parsing from tx not implemented yet
        }
    }
}

// ---------------------------------------------------------------------------
// Simulated client for devnet dry-run / demo mode
// ---------------------------------------------------------------------------

/// A simulated Phoenix client that logs quotes without sending transactions.
/// Used for testing, demos, and when the live SDK has dependency issues.
pub struct SimulatedPhoenixClient {
    pair: String,
}

impl SimulatedPhoenixClient {
    pub fn new(pair: &str) -> Self {
        info!(pair, "simulated Phoenix client initialized (no on-chain txs)");
        Self {
            pair: pair.to_string(),
        }
    }

    #[allow(dead_code)]
    pub async fn get_ladder(&self, _levels: usize) -> Result<OrderbookSnapshot> {
        // Return empty book — engine uses oracle price as fair value anyway
        Ok(OrderbookSnapshot {
            bids: Vec::new(),
            asks: Vec::new(),
        })
    }

    pub async fn cancel_and_place(&self, quotes: &[QuoteLevel]) -> Result<Vec<Fill>> {
        let bids: Vec<_> = quotes.iter().filter(|q| q.side == Side::Bid).collect();
        let asks: Vec<_> = quotes.iter().filter(|q| q.side == Side::Ask).collect();

        if quotes.is_empty() {
            debug!(pair = %self.pair, "SIM: cancelled all orders");
        } else {
            for b in &bids {
                debug!(
                    pair = %self.pair,
                    level = b.level,
                    price = format!("{:.4}", b.price),
                    size = format!("{:.4}", b.size),
                    "SIM: BID"
                );
            }
            for a in &asks {
                debug!(
                    pair = %self.pair,
                    level = a.level,
                    price = format!("{:.4}", a.price),
                    size = format!("{:.4}", a.size),
                    "SIM: ASK"
                );
            }
            info!(
                pair = %self.pair,
                bids = bids.len(),
                asks = asks.len(),
                best_bid = bids.first().map(|b| format!("{:.2}", b.price)).unwrap_or_default(),
                best_ask = asks.first().map(|a| format!("{:.2}", a.price)).unwrap_or_default(),
                "SIM: placed orders"
            );
        }

        Ok(Vec::new())
    }
}
