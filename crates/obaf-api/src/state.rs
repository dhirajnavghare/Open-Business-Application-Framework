use obaf_database::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::webhooks::dispatcher::dispatch_webhooks_background;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub jwt_secret: String,
    pub event_sender: broadcast::Sender<String>,
    pub http_client: reqwest::Client,
    pub version: &'static str,
}

impl AppState {
    pub fn new(db: DatabaseConnection, jwt_secret: impl Into<String>) -> Self {
        let (event_sender, _) = broadcast::channel(1024);
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            db,
            jwt_secret: jwt_secret.into(),
            event_sender,
            http_client,
            version: env!("CARGO_PKG_VERSION"),
        }
    }

    pub fn broadcast_event(&self, event_type: &str, payload: serde_json::Value) {
        // 1. Broadcast to internal WebSocket listeners
        let msg = serde_json::json!({
            "event": event_type,
            "data": payload,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
        .to_string();
        let _ = self.event_sender.send(msg);

        // 2. Dispatch to registered HTTP webhooks in the background
        let db = self.db.clone();
        let client = self.http_client.clone();
        let event_type_owned = event_type.to_string();
        tokio::spawn(async move {
            dispatch_webhooks_background(db, client, event_type_owned, payload).await;
        });
    }
}

pub type SharedState = Arc<AppState>;
