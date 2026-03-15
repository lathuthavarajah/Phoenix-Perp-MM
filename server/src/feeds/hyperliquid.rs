use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};

use crate::types::*;

const HL_WS_URL: &str = "wss://api.hyperliquid.xyz/ws";

/// Outer loop handles reconnection.
pub async fn run_hyperliquid_feed(pair: TradingPair, tx: tokio::sync::broadcast::Sender<WsMessage>) {
    loop {
        if let Err(e) = connect_and_stream(pair, tx.clone()).await {
            warn!("Hyperliquid feed ({:?}) disconnected: {e}. Reconnecting in 2s...", pair);
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
}

async fn connect_and_stream(
    pair: TradingPair,
    tx: tokio::sync::broadcast::Sender<WsMessage>,
) -> Result<()> {
    let coin = match pair {
        TradingPair::SolPerp => "SOL",
        TradingPair::BtcPerp => "BTC",
    };

    let (ws_stream, _) = connect_async(HL_WS_URL).await?;
    info!("Connected to Hyperliquid WS for {coin}");
    let (mut write, mut read) = ws_stream.split();

    write
        .send(Message::Text(
            json!({
                "method": "subscribe",
                "subscription": { "type": "l2Book", "coin": coin, "nSigFigs": 5, "nLevels": 20 }
            })
            .to_string(),
        ))
        .await?;

    write
        .send(Message::Text(
            json!({
                "method": "subscribe",
                "subscription": { "type": "activeAssetCtx", "coin": coin }
            })
            .to_string(),
        ))
        .await?;

    while let Some(msg) = read.next().await {
        let text = match msg? {
            Message::Text(t) => t,
            Message::Close(_) => return Err(anyhow!("WS closed")),
            _ => continue,
        };

        let v: Value = serde_json::from_str(&text)?;
        match v["channel"].as_str().unwrap_or("") {
            "l2Book" => {
                if let Some(ob) = parse_hl_orderbook(&v, pair) {
                    let _ = tx.send(WsMessage::Orderbook(ob));
                }
            }
            "activeAssetCtx" => {
                if let Some(fr) = parse_hl_funding(&v, pair) {
                    let _ = tx.send(WsMessage::Funding(fr));
                }
            }
            _ => {}
        }
    }
    Err(anyhow!("Hyperliquid stream ended"))
}

fn parse_hl_orderbook(v: &Value, pair: TradingPair) -> Option<Orderbook> {
    let data = &v["data"];
    let levels = data["levels"].as_array()?;
    let raw_bids = levels.first()?.as_array()?;
    let raw_asks = levels.get(1)?.as_array()?;
    let ts = data["time"].as_u64().unwrap_or(0);

    let parse_levels = |raw: &[Value]| -> Vec<OrderbookLevel> {
        raw.iter()
            .filter_map(|l| {
                let price: f64 = l["px"].as_str()?.parse().ok()?;
                let size: f64 = l["sz"].as_str()?.parse().ok()?;
                Some(OrderbookLevel {
                    price,
                    size,
                    notional: price * size,
                })
            })
            .collect()
    };

    Some(Orderbook {
        venue: Venue::Hyperliquid,
        pair,
        bids: parse_levels(raw_bids),
        asks: parse_levels(raw_asks),
        timestamp_ms: ts,
    })
}

fn parse_hl_funding(v: &Value, pair: TradingPair) -> Option<FundingRateSnapshot> {
    let ctx = &v["data"]["ctx"];
    let rate_hourly: f64 = ctx["funding"].as_str()?.parse().ok()?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    Some(FundingRateSnapshot {
        venue: Venue::Hyperliquid,
        pair,
        rate_hourly,
        rate_annualized: rate_hourly * 8760.0,
        next_funding_ms: 0,
        timestamp_ms: ts,
    })
}
