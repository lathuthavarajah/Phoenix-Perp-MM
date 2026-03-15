use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::debug;

/// Fetches SOL/USD price from Pyth via a public price API.
/// In production this would read directly from Solana RPC,
/// but for the demo we use the Pyth HTTP endpoint for simplicity.
pub struct OracleClient {
    cache: Arc<Mutex<Option<(f64, Instant)>>>,
    cache_ttl_ms: u64,
}

impl OracleClient {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(None)),
            cache_ttl_ms: 2000,
        }
    }

    pub async fn get_sol_price(&self) -> Result<f64> {
        // Check cache
        {
            let cache = self.cache.lock().await;
            if let Some((price, fetched_at)) = &*cache {
                if fetched_at.elapsed().as_millis() < self.cache_ttl_ms as u128 {
                    return Ok(*price);
                }
            }
        }

        let price = self.fetch_price().await?;

        let mut cache = self.cache.lock().await;
        *cache = Some((price, Instant::now()));

        debug!(price = price, "Oracle: SOL/USD updated");
        Ok(price)
    }

    async fn fetch_price(&self) -> Result<f64> {
        // Use Binance spot price as a reliable oracle proxy
        let client = reqwest::Client::new();
        let resp = client
            .get("https://api.binance.com/api/v3/ticker/price?symbol=SOLUSDT")
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await?;

        let data: serde_json::Value = resp.json().await?;
        let price_str = data["price"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing price field"))?;
        let price: f64 = price_str.parse()?;
        Ok(price)
    }
}
