use tracing::info;

use crate::feeds::{binance, drift, hyperliquid, phoenix};
use crate::metrics::{compute_eqs, compute_spread, estimate_slippage};
use crate::state::AppState;
use crate::types::{Side, TradingPair, WsMessage};

pub async fn start_feeds(state: AppState) {
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

    // Spawn one feed task per venue per pair
    for pair in [TradingPair::SolPerp, TradingPair::BtcPerp] {
        let tx = state.tx.clone();
        tokio::spawn(hyperliquid::run_hyperliquid_feed(pair, tx.clone()));
        tokio::spawn(binance::run_binance_feed(pair, tx.clone()));
        tokio::spawn(drift::run_drift_feed(pair, tx.clone()));
        tokio::spawn(phoenix::run_phoenix_feed(
            pair,
            tx.clone(),
            rpc_url.clone(),
        ));
    }

    // Cache maintenance + EQS recomputation task
    let state_clone = state.clone();
    tokio::spawn(async move {
        let mut rx = state_clone.tx.subscribe();
        while let Ok(msg) = rx.recv().await {
            match msg {
                WsMessage::Orderbook(ob) => {
                    {
                        let mut obs = state_clone.orderbooks.write().await;
                        obs.insert((ob.venue, ob.pair), ob);
                    }

                    // Recompute EQS on orderbook updates
                    let all_obs: Vec<_> =
                        state_clone.orderbooks.read().await.values().cloned().collect();
                    let spreads: Vec<_> =
                        all_obs.iter().filter_map(compute_spread).collect();
                    let slippages: Vec<_> = all_obs
                        .iter()
                        .filter_map(|o| estimate_slippage(o, Side::Buy, 100_000.0))
                        .collect();
                    let funding_vec: Vec<_> =
                        state_clone.funding_rates.read().await.values().cloned().collect();

                    for pair in [TradingPair::SolPerp, TradingPair::BtcPerp] {
                        let scores = compute_eqs(&spreads, &slippages, &funding_vec, pair);
                        if !scores.is_empty() {
                            let _ = state_clone.tx.send(WsMessage::Scores(scores));
                        }
                    }
                }
                WsMessage::Funding(fr) => {
                    let mut rates = state_clone.funding_rates.write().await;
                    rates.insert((fr.venue, fr.pair), fr);
                }
                WsMessage::Scores(_) => {}
            }
        }
    });

    info!("All feeds and aggregator started");
}
