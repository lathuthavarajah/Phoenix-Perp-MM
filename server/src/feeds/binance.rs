use reqwest::Client;
use serde::Deserialize;
use tokio::time::{interval, Duration};

use crate::types::*;

const BINANCE_BASE: &str = "https://fapi.binance.com";

#[derive(Deserialize)]
struct DepthResponse {
    bids: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PremiumIndex {
    last_funding_rate: String,
    next_funding_time: u64,
}

pub async fn run_binance_feed(pair: TradingPair, tx: tokio::sync::broadcast::Sender<WsMessage>) {
    let client = Client::new();
    let symbol = match pair {
        TradingPair::SolPerp => "SOLUSDT",
        TradingPair::BtcPerp => "BTCUSDT",
    };
    let mut tick = interval(Duration::from_secs(1));

    loop {
        tick.tick().await;

        let ob_url = format!("{BINANCE_BASE}/fapi/v1/depth?symbol={symbol}&limit=20");
        let fund_url = format!("{BINANCE_BASE}/fapi/v1/premiumIndex?symbol={symbol}");

        let (ob_res, fund_res) = tokio::join!(client.get(&ob_url).send(), client.get(&fund_url).send(),);

        if let Ok(resp) = ob_res {
            if let Ok(data) = resp.json::<DepthResponse>().await {
                let parse = |raw: Vec<[String; 2]>| -> Vec<OrderbookLevel> {
                    raw.into_iter()
                        .filter_map(|[p, s]| {
                            let price: f64 = p.parse().ok()?;
                            let size: f64 = s.parse().ok()?;
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
                    venue: Venue::Binance,
                    pair,
                    bids: parse(data.bids),
                    asks: parse(data.asks),
                    timestamp_ms: ts,
                }));
            }
        }

        if let Ok(resp) = fund_res {
            if let Ok(data) = resp.json::<PremiumIndex>().await {
                if let Ok(rate_8h) = data.last_funding_rate.parse::<f64>() {
                    let rate_hourly = rate_8h / 8.0;
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
                    let _ = tx.send(WsMessage::Funding(FundingRateSnapshot {
                        venue: Venue::Binance,
                        pair,
                        rate_hourly,
                        rate_annualized: rate_hourly * 8760.0,
                        next_funding_ms: data.next_funding_time,
                        timestamp_ms: ts,
                    }));
                }
            }
        }
    }
}
