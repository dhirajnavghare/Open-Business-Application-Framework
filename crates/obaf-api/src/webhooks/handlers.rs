use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use obaf_database::entities::webhook;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct CreateWebhookRequest {
    pub url: String,
    pub secret: Option<String>,
    pub event_types: Vec<String>, // e.g. ["order.created", "stock.low", "payment.received"]
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateWebhookRequest {
    pub url: Option<String>,
    pub secret: Option<String>,
    pub event_types: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

#[derive(Serialize, ToSchema)]
pub struct WebhookResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub url: String,
    pub event_types: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<webhook::Model> for WebhookResponse {
    fn from(m: webhook::Model) -> Self {
        let events = serde_json::from_str(&m.event_types).unwrap_or_default();
        Self {
            id: m.id,
            tenant_id: m.tenant_id,
            url: m.url,
            event_types: events,
            is_active: m.is_active,
            created_at: m.created_at,
        }
    }
}

/// Validates webhook target URL to mitigate SSRF and protocol manipulation vulnerabilities.
fn validate_webhook_url(raw_url: &str) -> Result<(), &'static str> {
    let parsed = reqwest::Url::parse(raw_url).map_err(|_| "Invalid webhook URL format")?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("Webhook URL must use http or https scheme");
    }
    let host = parsed
        .host_str()
        .ok_or("Webhook URL must contain a valid hostname")?;
    let host_lower = host.to_lowercase();

    // Prevent Server-Side Request Forgery (SSRF) to cloud metadata endpoints and local loopback
    if host_lower == "169.254.169.254"
        || host_lower == "metadata.google.internal"
        || host_lower == "100.100.100.200"
        || host_lower.ends_with(".internal")
        || host_lower == "localhost"
        || host_lower == "127.0.0.1"
        || host_lower == "0.0.0.0"
        || host_lower == "::1"
        || host_lower == "[::1]"
        || host_lower.starts_with("127.")
    {
        return Err(
            "Webhook destination address is prohibited (cloud metadata or loopback service)",
        );
    }

    // Prohibit private IP ranges (RFC 1918 & link-local)
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        match ip {
            std::net::IpAddr::V4(v4) => {
                if v4.is_loopback() || v4.is_private() || v4.is_link_local() {
                    return Err(
                        "Webhook destination address is prohibited (private or loopback IP range)",
                    );
                }
            }
            std::net::IpAddr::V6(v6) => {
                if v6.is_loopback() {
                    return Err("Webhook destination address is prohibited (IPv6 loopback)");
                }
            }
        }
    }

    Ok(())
}

#[utoipa::path(
    post,
    path = "/api/v1/webhooks",
    tag = "Webhooks",
    security(("bearer_auth" = [])),
    request_body = CreateWebhookRequest,
    responses(
        (status = 201, description = "Webhook registered successfully", body = WebhookResponse),
        (status = 400, description = "Invalid webhook URL", body = AuthErrorResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn register_webhook(
    State(state): State<SharedState>,
    auth: AuthUser,
    Json(payload): Json<CreateWebhookRequest>,
) -> Result<(StatusCode, Json<WebhookResponse>), (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin"])?;

    if let Err(err_msg) = validate_webhook_url(&payload.url) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: err_msg.to_string(),
            }),
        ));
    }

    let hook_id = Uuid::new_v4();
    let secret = payload.secret.unwrap_or_else(|| Uuid::new_v4().to_string());
    let events_json = serde_json::to_string(&payload.event_types).unwrap_or_default();

    let new_hook = webhook::ActiveModel {
        id: Set(hook_id),
        tenant_id: Set(auth.tenant_id.as_uuid()),
        url: Set(payload.url),
        secret: Set(secret),
        event_types: Set(events_json),
        is_active: Set(true),
        created_at: Set(Utc::now()),
    };

    let inserted = new_hook.insert(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to register webhook: {}", e),
            }),
        )
    })?;

    Ok((StatusCode::CREATED, Json(inserted.into())))
}

#[utoipa::path(
    get,
    path = "/api/v1/webhooks",
    tag = "Webhooks",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of active webhooks", body = Vec<WebhookResponse>),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn list_webhooks(
    State(state): State<SharedState>,
    auth: AuthUser,
) -> Result<Json<Vec<WebhookResponse>>, (StatusCode, Json<AuthErrorResponse>)> {
    let hooks = webhook::Entity::find()
        .filter(webhook::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .limit(100)
        .all(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?;

    let resp = hooks.into_iter().map(Into::into).collect();
    Ok(Json(resp))
}

#[utoipa::path(
    post,
    path = "/api/v1/webhooks/{id}/test",
    tag = "Webhooks",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Webhook ID")
    ),
    responses(
        (status = 200, description = "Test ping triggered"),
        (status = 404, description = "Webhook not found", body = AuthErrorResponse)
    )
)]
pub async fn test_webhook(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<AuthErrorResponse>)> {
    let hook = webhook::Entity::find()
        .filter(webhook::Column::Id.eq(id))
        .filter(webhook::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: "Webhook not found".to_string(),
                }),
            )
        })?;

    state.broadcast_tenant_event(
        auth.tenant_id.as_uuid(),
        "webhook.test_ping",
        serde_json::json!({
            "webhook_id": hook.id,
            "target_url": hook.url,
            "message": "OBAF Webhook test ping successfully triggered"
        }),
    );

    Ok(Json(serde_json::json!({
        "status": "ping_dispatched",
        "webhook_id": hook.id,
        "target_url": hook.url
    })))
}

#[utoipa::path(
    get,
    path = "/api/v1/webhooks/{id}",
    tag = "Webhooks",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Webhook ID")
    ),
    responses(
        (status = 200, description = "Webhook details", body = WebhookResponse),
        (status = 404, description = "Webhook not found", body = AuthErrorResponse)
    )
)]
pub async fn get_webhook(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<WebhookResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let hook = webhook::Entity::find()
        .filter(webhook::Column::Id.eq(id))
        .filter(webhook::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: "Webhook not found".to_string(),
                }),
            )
        })?;

    Ok(Json(hook.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/webhooks/{id}",
    tag = "Webhooks",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Webhook ID")
    ),
    request_body = UpdateWebhookRequest,
    responses(
        (status = 200, description = "Webhook updated successfully", body = WebhookResponse),
        (status = 404, description = "Webhook not found", body = AuthErrorResponse)
    )
)]
pub async fn update_webhook(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateWebhookRequest>,
) -> Result<Json<WebhookResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin"])?;

    let hook = webhook::Entity::find()
        .filter(webhook::Column::Id.eq(id))
        .filter(webhook::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: "Webhook not found".to_string(),
                }),
            )
        })?;

    let mut active: webhook::ActiveModel = hook.into();

    if let Some(url) = payload.url {
        if let Err(err_msg) = validate_webhook_url(&url) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(AuthErrorResponse {
                    error: err_msg.to_string(),
                }),
            ));
        }
        active.url = Set(url);
    }
    if let Some(secret) = payload.secret {
        active.secret = Set(secret);
    }
    if let Some(events) = payload.event_types {
        let events_json = serde_json::to_string(&events).unwrap_or_default();
        active.event_types = Set(events_json);
    }
    if let Some(is_active) = payload.is_active {
        active.is_active = Set(is_active);
    }

    let updated = active.update(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to update webhook: {}", e),
            }),
        )
    })?;

    Ok(Json(updated.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/webhooks/{id}",
    tag = "Webhooks",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Webhook ID")
    ),
    responses(
        (status = 200, description = "Webhook deleted successfully"),
        (status = 404, description = "Webhook not found", body = AuthErrorResponse)
    )
)]
pub async fn delete_webhook(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin"])?;

    let hook = webhook::Entity::find()
        .filter(webhook::Column::Id.eq(id))
        .filter(webhook::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: "Webhook not found".to_string(),
                }),
            )
        })?;

    let active: webhook::ActiveModel = hook.into();
    active.delete(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to delete webhook: {}", e),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({
        "status": "deleted",
        "webhook_id": id
    })))
}
