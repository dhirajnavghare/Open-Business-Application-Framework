use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use obaf_database::entities::subscription;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct CreateSubscriptionRequest {
    pub customer_id: Uuid,
    pub plan_name: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub amount: Decimal,
    pub billing_cycle: String, // "MONTHLY", "DAILY", "ANNUAL"
    pub metadata: Option<serde_json::Value>,
}

#[derive(Serialize, ToSchema)]
pub struct SubscriptionResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub customer_id: Uuid,
    pub plan_name: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub status: String,
    pub amount: Decimal,
    pub billing_cycle: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateSubscriptionStatusRequest {
    pub status: String, // "ACTIVE", "PAUSED", "EXPIRED", "CANCELLED"
}

impl From<subscription::Model> for SubscriptionResponse {
    fn from(m: subscription::Model) -> Self {
        let meta = serde_json::from_str(&m.metadata).unwrap_or(serde_json::json!({}));
        Self {
            id: m.id,
            tenant_id: m.tenant_id,
            customer_id: m.customer_id,
            plan_name: m.plan_name,
            start_date: m.start_date,
            end_date: m.end_date,
            status: m.status,
            amount: m.amount,
            billing_cycle: m.billing_cycle,
            metadata: meta,
            created_at: m.created_at,
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/subscriptions",
    tag = "Subscriptions",
    security(("bearer_auth" = [])),
    request_body = CreateSubscriptionRequest,
    responses(
        (status = 201, description = "Subscription created successfully", body = SubscriptionResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn create_subscription(
    State(state): State<SharedState>,
    auth: AuthUser,
    Json(payload): Json<CreateSubscriptionRequest>,
) -> Result<(StatusCode, Json<SubscriptionResponse>), (StatusCode, Json<AuthErrorResponse>)> {
    let sub_id = Uuid::new_v4();
    let now = Utc::now();
    let meta_str = payload
        .metadata
        .map(|v| v.to_string())
        .unwrap_or_else(|| "{}".to_string());

    let new_sub = subscription::ActiveModel {
        id: Set(sub_id),
        tenant_id: Set(auth.tenant_id.as_uuid()),
        customer_id: Set(payload.customer_id),
        plan_name: Set(payload.plan_name),
        start_date: Set(payload.start_date),
        end_date: Set(payload.end_date),
        status: Set("ACTIVE".to_string()),
        amount: Set(payload.amount),
        billing_cycle: Set(payload.billing_cycle),
        metadata: Set(meta_str),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let inserted = new_sub.insert(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to create subscription: {}", e),
            }),
        )
    })?;

    Ok((StatusCode::CREATED, Json(inserted.into())))
}

#[utoipa::path(
    get,
    path = "/api/v1/subscriptions",
    tag = "Subscriptions",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of subscriptions", body = Vec<SubscriptionResponse>),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn list_subscriptions(
    State(state): State<SharedState>,
    auth: AuthUser,
) -> Result<Json<Vec<SubscriptionResponse>>, (StatusCode, Json<AuthErrorResponse>)> {
    let subs = subscription::Entity::find()
        .filter(subscription::Column::TenantId.eq(auth.tenant_id.as_uuid()))
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

    let resp = subs.into_iter().map(Into::into).collect();
    Ok(Json(resp))
}

#[utoipa::path(
    patch,
    path = "/api/v1/subscriptions/{id}/status",
    tag = "Subscriptions",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Subscription ID")
    ),
    request_body = UpdateSubscriptionStatusRequest,
    responses(
        (status = 200, description = "Subscription status updated", body = SubscriptionResponse),
        (status = 404, description = "Subscription not found", body = AuthErrorResponse)
    )
)]
pub async fn update_status(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateSubscriptionStatusRequest>,
) -> Result<Json<SubscriptionResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let sub = subscription::Entity::find()
        .filter(subscription::Column::Id.eq(id))
        .filter(subscription::Column::TenantId.eq(auth.tenant_id.as_uuid()))
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
                    error: "Subscription not found".to_string(),
                }),
            )
        })?;

    let mut active: subscription::ActiveModel = sub.into();
    active.status = Set(payload.status);
    active.updated_at = Set(Utc::now());

    let updated = active.update(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to update subscription status: {}", e),
            }),
        )
    })?;

    Ok(Json(updated.into()))
}
