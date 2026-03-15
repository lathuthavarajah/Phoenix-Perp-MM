use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use tracing::info;

use crate::state::AppState;
use crate::types::WsMessage;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    // Send current snapshot so new clients see data immediately
    {
        let orderbooks = state.orderbooks.read().await;
        for ob in orderbooks.values() {
            let json = serde_json::to_string(&WsMessage::Orderbook(ob.clone())).unwrap();
            if sender
                .send(axum::extract::ws::Message::Text(json))
                .await
                .is_err()
            {
                return;
            }
        }
    }
    {
        let funding = state.funding_rates.read().await;
        for fr in funding.values() {
            let json = serde_json::to_string(&WsMessage::Funding(fr.clone())).unwrap();
            if sender
                .send(axum::extract::ws::Message::Text(json))
                .await
                .is_err()
            {
                return;
            }
        }
    }

    // Forward broadcast messages to this client
    let send_task = tokio::spawn(async move {
        while let Ok(ws_msg) = rx.recv().await {
            let json = serde_json::to_string(&ws_msg).unwrap();
            if sender
                .send(axum::extract::ws::Message::Text(json))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Drain incoming frames from client (read-only clients)
    while receiver.next().await.is_some() {}

    send_task.abort();
    info!("WebSocket client disconnected");
}
