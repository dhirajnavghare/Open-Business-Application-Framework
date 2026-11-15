use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use obaf_database::entities::webhook;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
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

#[utoipa::path(
    post,
    path = "/api/v1/webhooks",
    tag = "Webhooks",
    security(("bearer_auth" = [])),
    request_body = CreateWebhookRequest,
    responses(
        (status = 201, description = "Webhook registered successfully", body = WebhookResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn register_webhook(
    State(state): State<SharedState>,
    auth: AuthUser,
    Json(payload): Json<CreateWebhookRequest>,
) -> Result<(StatusCode, Json<WebhookResponse>), (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin"])?;

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

    state.broadcast_event(
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
