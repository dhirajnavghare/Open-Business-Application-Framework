use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use tracing::info;

use crate::state::SharedState;

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<SharedState>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: SharedState) {
    info!("New real-time client connected to WebSocket event hub");
    let mut receiver = state.event_sender.subscribe();

    while let Ok(msg_text) = receiver.recv().await {
        if socket.send(Message::Text(msg_text)).await.is_err() {
            // Client disconnected
            break;
        }
    }
}
