use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tracing::{error, info, warn};

use crate::types::*;

// Phoenix Legacy SOL/USDC market on mainnet
const PHOENIX_SOL_USDC_MARKET: &str = "4DoNfFBfF7UokCC2FQzriy7yHK6DY6NVdYpuekQ5pRgg";

pub async fn run_phoenix_feed(
    pair: TradingPair,
    tx: tokio::sync::broadcast::Sender<WsMessage>,
    rpc_url: String,
) {
    let client = RpcClient::new(rpc_url);
    let market_pubkey = match Pubkey::from_str(PHOENIX_SOL_USDC_MARKET) {
        Ok(pk) => pk,
        Err(e) => {
            error!("Invalid Phoenix market pubkey: {e}");
            return;
        }
    };

    info!("Starting Phoenix feed for {:?}", pair);
    let mut tick = tokio::time::interval(tokio::time::Duration::from_millis(500));

    loop {
        tick.tick().await;

        match client.get_account(&market_pubkey).await {
            Ok(account) => {
                if let Some(ob) = decode_phoenix_account(&account.data, pair) {
                    let _ = tx.send(WsMessage::Orderbook(ob));
                } else {
                    warn!("Phoenix account fetched but decoder returned None");
                }
            }
            Err(e) => {
                error!("Phoenix RPC error: {e}");
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    }
}

/// Decode Phoenix market account binary data into a normalized Orderbook.
///
/// Phoenix Perps is in private beta with no public SDK. This stub returns None,
/// causing the Phoenix panel to show "connecting..." in the frontend.
///
/// When the Phoenix Perps SDK becomes public, implement using:
/// 1. phoenix-types crate for Market::deserialize
/// 2. market.get_ladder(20) for bids/asks
/// 3. Tick size scaling from market metadata
fn decode_phoenix_account(_data: &[u8], _pair: TradingPair) -> Option<Orderbook> {
    // TODO: Implement using phoenix-types deserialization
    //
    // use phoenix_types::market::Market;
    // let market = Market::deserialize(data)?;
    // let ladder = market.get_ladder(20);
    // ...
    None
}
