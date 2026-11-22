use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use obaf_database::entities::{customer, subscription};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set};
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
    auth.require_role(&["admin", "manager", "staff"])?;

    if payload.amount < Decimal::ZERO {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: "Subscription amount cannot be negative".to_string(),
            }),
        ));
    }

    // BOLA/IDOR Defense: Verify customer belongs to current tenant
    let cust_exists = customer::Entity::find_by_id(payload.customer_id)
        .filter(customer::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database query error verifying customer: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: "A database error occurred".to_string(),
                }),
            )
        })?;

    if cust_exists.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: format!(
                    "Customer '{}' not found for current tenant",
                    payload.customer_id
                ),
            }),
        ));
    }

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
        tracing::error!("Failed to create subscription: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: "Failed to create subscription".to_string(),
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
    auth.require_role(&["admin", "manager"])?;

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

#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/{id}/check-in",
    tag = "Subscriptions",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Subscription ID")
    ),
    responses(
        (status = 200, description = "Member checked in successfully", body = serde_json::Value),
        (status = 400, description = "Subscription is not active", body = AuthErrorResponse),
        (status = 404, description = "Subscription not found", body = AuthErrorResponse)
    )
)]
pub async fn check_in_member(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager", "staff", "cashier"])?;

    let sub = subscription::Entity::find()
        .filter(subscription::Column::Id.eq(id))
        .filter(subscription::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .one(&state.db)
        .await
        .map_err(|e| internal_err(format!("Database error: {}", e)))?
        .ok_or_else(|| not_found_err("Subscription not found"))?;

    if sub.status.to_uppercase() != "ACTIVE" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: format!("Cannot check in: subscription is {}", sub.status),
            }),
        ));
    }

    // Parse existing metadata, update check-in count and timestamp
    let mut meta: serde_json::Value =
        serde_json::from_str(&sub.metadata).unwrap_or(serde_json::json!({}));
    let total_check_ins = meta
        .get("check_in_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        + 1;
    let now = Utc::now();
    meta["check_in_count"] = serde_json::json!(total_check_ins);
    meta["last_check_in_at"] = serde_json::json!(now.to_rfc3339());

    let mut active: subscription::ActiveModel = sub.clone().into();
    active.metadata = Set(meta.to_string());
    active.updated_at = Set(now);

    let _ = active
        .update(&state.db)
        .await
        .map_err(|e| internal_err(format!("Update failed: {}", e)))?;

    state.broadcast_tenant_event(
        auth.tenant_id.as_uuid(),
        "subscription.check_in",
        serde_json::json!({
            "subscription_id": sub.id,
            "customer_id": sub.customer_id,
            "plan_name": sub.plan_name,
            "total_check_ins": total_check_ins,
            "timestamp": now.to_rfc3339()
        }),
    );

    Ok(Json(serde_json::json!({
        "status": "checked_in",
        "subscription_id": sub.id,
        "customer_id": sub.customer_id,
        "plan_name": sub.plan_name,
        "total_check_ins": total_check_ins,
        "last_check_in_at": now.to_rfc3339()
    })))
}

#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/{id}",
    tag = "Subscriptions",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Subscription ID")
    ),
    responses(
        (status = 200, description = "Subscription details", body = SubscriptionResponse),
        (status = 404, description = "Subscription not found", body = AuthErrorResponse)
    )
)]
pub async fn get_subscription(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<SubscriptionResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let sub = subscription::Entity::find()
        .filter(subscription::Column::Id.eq(id))
        .filter(subscription::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .one(&state.db)
        .await
        .map_err(|e| internal_err(format!("Database error: {}", e)))?
        .ok_or_else(|| not_found_err("Subscription not found"))?;

    Ok(Json(sub.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/subscriptions/{id}",
    tag = "Subscriptions",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Subscription ID")
    ),
    responses(
        (status = 200, description = "Subscription deleted"),
        (status = 404, description = "Subscription not found", body = AuthErrorResponse)
    )
)]
pub async fn delete_subscription(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager"])?;

    let sub = subscription::Entity::find()
        .filter(subscription::Column::Id.eq(id))
        .filter(subscription::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .one(&state.db)
        .await
        .map_err(|e| internal_err(format!("Database error: {}", e)))?
        .ok_or_else(|| not_found_err("Subscription not found"))?;

    let active: subscription::ActiveModel = sub.into();
    active
        .delete(&state.db)
        .await
        .map_err(|e| internal_err(format!("Delete failed: {}", e)))?;

    Ok(Json(serde_json::json!({
        "status": "deleted",
        "subscription_id": id
    })))
}

fn internal_err(msg: String) -> (StatusCode, Json<AuthErrorResponse>) {
    tracing::error!("Internal error in subscriptions: {}", msg);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(AuthErrorResponse {
            error: "A database error occurred".to_string(),
        }),
    )
}

fn not_found_err(msg: &str) -> (StatusCode, Json<AuthErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(AuthErrorResponse {
            error: msg.to_string(),
        }),
    )
}
