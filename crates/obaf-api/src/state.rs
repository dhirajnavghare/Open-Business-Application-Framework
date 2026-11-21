use axum::http::StatusCode;
use obaf_core::{TokenBucketRateLimiter, TtlCache};
use obaf_database::{DatabaseConnection, ReadWriteRouter};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

use crate::webhooks::dispatcher::dispatch_webhooks_background;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub router: ReadWriteRouter,
    pub jwt_secret: String,
    pub event_sender: broadcast::Sender<String>,
    pub http_client: reqwest::Client,
    pub version: &'static str,
    pub cache: TtlCache<String, String>,
    pub idempotency_cache: TtlCache<String, (StatusCode, serde_json::Value)>,
    pub rate_limiter: TokenBucketRateLimiter,
    pub superadmin: crate::superadmin::SuperAdminConfig,
}

impl AppState {
    pub fn new(db: DatabaseConnection, jwt_secret: impl Into<String>) -> Self {
        let router = ReadWriteRouter::new(db.clone());
        Self::with_router(db, router, jwt_secret)
    }

    pub fn with_router(
        db: DatabaseConnection,
        router: ReadWriteRouter,
        jwt_secret: impl Into<String>,
    ) -> Self {
        let (event_sender, _) = broadcast::channel(1024);
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        // System Design Primer: in-memory Cache-Aside store (10m TTL, 10k capacity)
        let cache = TtlCache::new(Duration::from_secs(600), 10_000);

        // System Design Primer: Idempotency deduplication store (24h TTL, 50k capacity)
        let idempotency_cache = TtlCache::new(Duration::from_secs(86_400), 50_000);

        // System Design Primer: Token Bucket Rate Limiter (burst 100 requests, 50 req/sec refill)
        let rate_limiter = TokenBucketRateLimiter::new(100.0, 50.0);

        // Platform Super Administrator single-user root credentials
        let superadmin = crate::superadmin::SuperAdminConfig::from_env();

        Self {
            db,
            router,
            jwt_secret: jwt_secret.into(),
            event_sender,
            http_client,
            version: env!("CARGO_PKG_VERSION"),
            cache,
            idempotency_cache,
            rate_limiter,
            superadmin,
        }
    }

    /// Set custom Super Admin configuration (useful for testing)
    pub fn with_superadmin(mut self, config: crate::superadmin::SuperAdminConfig) -> Self {
        self.superadmin = config;
        self
    }

    /// Access database connection pool for read operations (queries, reports, searches).
    /// Load-balances across registered read replicas or falls back to primary.
    pub fn reader(&self) -> &DatabaseConnection {
        self.router.reader()
    }

    /// Access primary database connection pool for write operations (inserts, updates, deletes).
    pub fn writer(&self) -> &DatabaseConnection {
        self.router.writer()
    }

    /// Broadcast a real-time event scoped to a specific tenant (WebSocket + Webhooks)
    pub fn broadcast_tenant_event(
        &self,
        tenant_id: uuid::Uuid,
        event_type: &str,
        payload: serde_json::Value,
    ) {
        // 1. Broadcast to internal WebSocket listeners with tenant tagging
        let msg = serde_json::json!({
            "event": event_type,
            "tenant_id": tenant_id,
            "data": payload,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
        .to_string();
        let _ = self.event_sender.send(msg);

        // 2. Dispatch to registered HTTP webhooks in the background (tenant isolated)
        let db = self.db.clone();
        let client = self.http_client.clone();
        let event_type_owned = event_type.to_string();
        tokio::spawn(async move {
            dispatch_webhooks_background(db, client, Some(tenant_id), event_type_owned, payload)
                .await;
        });
    }

    pub fn broadcast_event(&self, event_type: &str, payload: serde_json::Value) {
        let tenant_id = payload
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .and_then(|s| uuid::Uuid::parse_str(s).ok());

        if let Some(tid) = tenant_id {
            self.broadcast_tenant_event(tid, event_type, payload);
        } else {
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
                dispatch_webhooks_background(db, client, None, event_type_owned, payload).await;
            });
        }
    }
}

pub type SharedState = Arc<AppState>;
