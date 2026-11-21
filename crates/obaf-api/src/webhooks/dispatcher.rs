use hmac::{Hmac, Mac};
use obaf_database::entities::webhook;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use sha2::Sha256;
use std::time::Duration;
use tracing::{debug, error, info};

type HmacSha256 = Hmac<Sha256>;

/// Computes an HMAC-SHA256 hex digest for webhook payload verification.
pub fn compute_hmac_signature(secret: &str, body: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can accept key of any size");
    mac.update(body.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Constant-time verification of HMAC-SHA256 signature to mitigate timing attacks.
pub fn verify_hmac_signature(secret: &str, body: &str, provided_sig: &str) -> bool {
    let expected = compute_hmac_signature(secret, body);
    obaf_core::constant_time_compare(&expected, provided_sig)
}

/// Dispatches an event asynchronously to registered webhooks matching the event type and tenant.
pub async fn dispatch_webhooks_background(
    db: obaf_database::DatabaseConnection,
    client: reqwest::Client,
    tenant_id: Option<uuid::Uuid>,
    event_type: String,
    payload: serde_json::Value,
) {
    let mut query = webhook::Entity::find().filter(webhook::Column::IsActive.eq(true));

    if let Some(tid) = tenant_id {
        query = query.filter(webhook::Column::TenantId.eq(tid));
    }

    let active_hooks = match query.all(&db).await {
        Ok(hooks) => hooks,
        Err(e) => {
            error!(
                "Failed to fetch active webhooks for event {}: {}",
                event_type, e
            );
            return;
        }
    };

    if active_hooks.is_empty() {
        return;
    }

    let now = chrono::Utc::now();
    let timestamp_str = now.to_rfc3339();
    let timestamp_epoch = now.timestamp();

    for hook in active_hooks {
        // Parse event_types array from JSON string
        let events: Vec<String> = serde_json::from_str(&hook.event_types).unwrap_or_default();
        let matches = events
            .iter()
            .any(|e| e == "*" || e == "all" || e == &event_type);

        if !matches {
            continue;
        }

        let body = serde_json::json!({
            "event": event_type,
            "data": payload,
            "timestamp": timestamp_str,
            "webhook_id": hook.id
        });

        let body_str = body.to_string();
        let signature = compute_hmac_signature(&hook.secret, &body_str);

        let hook_url = hook.url.clone();
        let client = client.clone();
        let event_name = event_type.clone();

        tokio::spawn(async move {
            debug!("Dispatching webhook event {} to {}", event_name, hook_url);
            match client
                .post(&hook_url)
                .header("Content-Type", "application/json")
                .header("X-OBAF-Signature", format!("sha256={}", signature))
                .header("X-OBAF-Event", &event_name)
                .header("X-OBAF-Timestamp", timestamp_epoch.to_string())
                .body(body_str)
                .timeout(Duration::from_secs(5))
                .send()
                .await
            {
                Ok(resp) => {
                    info!(
                        "Webhook delivered to {} with status {}",
                        hook_url,
                        resp.status()
                    );
                }
                Err(err) => {
                    debug!("Webhook delivery failed to {}: {}", hook_url, err);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_sha256_computation() {
        let secret = "test-secret-key-123";
        let body = r#"{"event":"order.created","amount":100}"#;
        let sig1 = compute_hmac_signature(secret, body);
        let sig2 = compute_hmac_signature(secret, body);

        // Signatures must be deterministic and exactly 64 hex characters (256 bits)
        assert_eq!(sig1, sig2);
        assert_eq!(sig1.len(), 64);

        // Different secret produces different signature
        let sig_diff = compute_hmac_signature("different-key", body);
        assert_ne!(sig1, sig_diff);
    }
}
