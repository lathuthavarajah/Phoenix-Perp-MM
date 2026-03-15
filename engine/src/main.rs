mod config;
mod executor;
mod funding_monitor;
mod hedge_engine;
mod oracle_client;
mod perp_client;
mod risk_monitor;
mod spot_client;
mod state_server;
mod types;

#[cfg(test)]
mod tests;

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::info;

use crate::types::{EngineMode, EngineState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "phoenix_arb_engine=debug,info".parse().unwrap()),
        )
        .init();

    info!("Starting Phoenix Arb Engine");

    let config = config::load_config();
    let port = config.engine_http_port;

    // Shared state
    let engine_state = Arc::new(RwLock::new(EngineState {
        position: None,
        margin: None,
        current_funding: vec![],
        sol_price: 0.0,
        last_signal: None,
        uptime_seconds: 0,
        alerts: vec![],
        mode: if config.use_devnet {
            EngineMode::Devnet
        } else {
            EngineMode::Paper
        },
    }));

    // Clients
    let oracle = Arc::new(oracle_client::OracleClient::new());
    let spot_client = Arc::new(spot_client::SpotClient::new());
    let perp_client: Arc<dyn perp_client::PerpClientTrait> =
        Arc::new(perp_client::MockPerpClient::new(oracle.clone()));
    let executor = Arc::new(executor::Executor::new(
        spot_client.clone(),
        perp_client.clone(),
    ));

    // Channels
    let (signal_tx, signal_rx) = broadcast::channel::<types::FundingSignal>(16);
    let (alerts_tx, _alerts_rx) = broadcast::channel::<String>(64);

    // Modules
    let funding_monitor = funding_monitor::FundingMonitor::new(
        perp_client.clone(),
        config.clone(),
        signal_tx,
        engine_state.clone(),
    );

    let hedge_engine = hedge_engine::HedgeEngine::new(
        oracle.clone(),
        executor.clone(),
        config.clone(),
        engine_state.clone(),
        signal_rx,
    );

    let risk_monitor = risk_monitor::RiskMonitor::new(
        perp_client.clone(),
        oracle.clone(),
        executor.clone(),
        config.clone(),
        engine_state.clone(),
        alerts_tx,
    );

    let app_state = state_server::AppState {
        engine_state: engine_state.clone(),
        executor: executor.clone(),
        oracle: oracle.clone(),
        perp_client: perp_client.clone(),
        config: config.clone(),
    };

    // Track uptime
    let uptime_state = engine_state.clone();
    let start = std::time::Instant::now();

    info!(port = port, "Spawning all async tasks");

    tokio::select! {
        r = funding_monitor.run() => {
            tracing::error!(result = ?r, "Funding monitor exited");
        }
        r = hedge_engine.run() => {
            tracing::error!(result = ?r, "Hedge engine exited");
        }
        r = risk_monitor.run() => {
            tracing::error!(result = ?r, "Risk monitor exited");
        }
        r = state_server::serve(app_state, port) => {
            tracing::error!(result = ?r, "State server exited");
        }
        _ = async {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
            loop {
                interval.tick().await;
                let mut state = uptime_state.write().await;
                state.uptime_seconds = start.elapsed().as_secs();
            }
        } => {}
    }

    Ok(())
}
