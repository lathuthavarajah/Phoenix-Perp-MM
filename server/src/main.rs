mod aggregator;
mod broadcast;
mod feeds;
mod metrics;
mod state;
mod types;

use axum::{routing::get, Router};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

use crate::broadcast::ws_handler;
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("phoenix_exec_quality=debug".parse()?)
                .add_directive("tower_http=info".parse()?),
        )
        .init();

    let state = AppState::new();
    aggregator::start_feeds(state.clone()).await;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/health", get(|| async { "ok" }))
        .with_state(state)
        .layer(cors);

    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
