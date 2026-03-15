use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::executor::Executor;
use crate::oracle_client::OracleClient;
use crate::perp_client::PerpClientTrait;
use crate::types::{EngineConfig, EngineState};

#[derive(Clone)]
pub struct AppState {
    pub engine_state: Arc<RwLock<EngineState>>,
    pub executor: Arc<Executor>,
    pub oracle: Arc<OracleClient>,
    pub perp_client: Arc<dyn PerpClientTrait>,
    pub config: EngineConfig,
}

pub async fn serve(app_state: AppState, port: u16) -> anyhow::Result<()> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/state", get(get_state))
        .route("/health", get(health))
        .route("/open", post(open_position))
        .route("/close", post(close_position))
        .layer(cors)
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!(port = port, "State server listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn get_state(State(state): State<AppState>) -> Json<EngineState> {
    let engine_state = state.engine_state.read().await;
    Json(engine_state.clone())
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn open_position(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let has_position = state.engine_state.read().await.position.is_some();
    if has_position {
        return Ok(Json(serde_json::json!({
            "success": false,
            "error": "Position already open"
        })));
    }

    let sol_price = state
        .oracle
        .get_sol_price()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let spot_size = state.config.position_size_usdc / sol_price;
    let perp_size = -spot_size;
    let collateral = spot_size * sol_price / state.config.max_leverage;

    match state
        .executor
        .open_basis_position(spot_size, perp_size, collateral, sol_price)
        .await
    {
        Ok(position) => {
            let mut engine_state = state.engine_state.write().await;
            engine_state.position = Some(position);
            Ok(Json(serde_json::json!({ "success": true })))
        }
        Err(e) => Ok(Json(serde_json::json!({
            "success": false,
            "error": e.to_string()
        }))),
    }
}

async fn close_position(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let position = state.engine_state.read().await.position.clone();

    match position {
        Some(pos) => {
            match state.executor.close_basis_position(&pos).await {
                Ok(()) => {
                    let mut engine_state = state.engine_state.write().await;
                    engine_state.position = None;
                    engine_state.margin = None;
                    Ok(Json(serde_json::json!({ "success": true })))
                }
                Err(e) => Ok(Json(serde_json::json!({
                    "success": false,
                    "error": e.to_string()
                }))),
            }
        }
        None => Ok(Json(serde_json::json!({
            "success": false,
            "error": "No position to close"
        }))),
    }
}
