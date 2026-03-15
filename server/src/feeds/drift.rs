use reqwest::Client;
use serde::Deserialize;
use tokio::time::{interval, Duration};

use crate::types::*;

const DRIFT_DLOB_BASE: &str = "https://dlob.drift.trade";

#[derive(Deserialize)]
struct DriftLevel {
    price: String,
    size: String,
}

#[derive(Deserialize)]
struct DriftOrderbook {
    bids: Vec<DriftLevel>,
    asks: Vec<DriftLevel>,
}

pub async fn run_drift_feed(pair: TradingPair, tx: tokio::sync::broadcast::Sender<WsMessage>) {
    let client = Client::new();
    let market_index: u32 = match pair {
        TradingPair::SolPerp => 0,
        TradingPair::BtcPerp => 1,
    };
    let mut tick = interval(Duration::from_millis(2000));

    loop {
        tick.tick().await;

        let url = format!(
            "{DRIFT_DLOB_BASE}/l2?marketIndex={market_index}&marketType=perp&depth=20"
        );

        if let Ok(resp) = client.get(&url).send().await {
            if let Ok(data) = resp.json::<DriftOrderbook>().await {
                let parse = |levels: Vec<DriftLevel>| -> Vec<OrderbookLevel> {
                    levels
                        .into_iter()
                        .filter_map(|l| {
                            let price: f64 = l.price.parse().ok()?;
                            let size: f64 = l.size.parse().ok()?;
                            Some(OrderbookLevel {
                                price,
                                size,
                                notional: price * size,
                            })
                        })
                        .collect()
                };
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let _ = tx.send(WsMessage::Orderbook(Orderbook {
                    venue: Venue::Drift,
                    pair,
                    bids: parse(data.bids),
                    asks: parse(data.asks),
                    timestamp_ms: ts,
                }));
            }
        }
    }
}
