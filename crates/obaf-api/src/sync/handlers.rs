use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use obaf_database::entities::{booking, order, payment, product};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

/// An offline transaction or state transition queued on a client device.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct SyncAction {
    pub client_action_id: Uuid,
    /// "create_order", "check_in_booking", "record_payment"
    pub action_type: String,
    pub payload: serde_json::Value,
    pub client_timestamp: DateTime<Utc>,
}

/// Outcome of processing a single offline synchronization action.
#[derive(Serialize, ToSchema)]
pub struct SyncActionResult {
    pub client_action_id: Uuid,
    /// "applied", "duplicate_ignored", or "failed"
    pub status: String,
    pub entity_id: Option<Uuid>,
    pub message: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct SyncPushRequest {
    pub actions: Vec<SyncAction>,
}

#[derive(Serialize, ToSchema)]
pub struct SyncPushResponse {
    pub processed: usize,
    pub results: Vec<SyncActionResult>,
}

#[derive(Deserialize, IntoParams)]
pub struct SyncPullQuery {
    pub since: Option<DateTime<Utc>>,
}

#[derive(Serialize, ToSchema)]
pub struct SyncPullResponse {
    pub sync_cursor: DateTime<Utc>,
    pub products_count: usize,
    pub orders_count: usize,
    pub bookings_count: usize,
    pub products: Vec<serde_json::Value>,
    pub orders: Vec<serde_json::Value>,
    pub bookings: Vec<serde_json::Value>,
}

#[utoipa::path(
    post,
    path = "/api/v1/sync/push",
    tag = "Sync",
    security(("bearer_auth" = [])),
    request_body = SyncPushRequest,
    responses(
        (status = 200, description = "Offline batch processed with idempotency tracking", body = SyncPushResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn sync_push(
    auth: AuthUser,
    State(state): State<SharedState>,
    Json(payload): Json<SyncPushRequest>,
) -> Result<Json<SyncPushResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager", "staff", "cashier"])?;

    let mut results = Vec::with_capacity(payload.actions.len());

    for action in payload.actions {
        let action_id = action.client_action_id;
        match action.action_type.as_str() {
            "create_order" => {
                let order_num = action
                    .payload
                    .get("order_number")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("OFFLINE-{}", action_id.simple()));

                // Idempotency check: does this order number already exist for this tenant?
                let existing = order::Entity::find()
                    .filter(order::Column::TenantId.eq(auth.tenant_id.0))
                    .filter(order::Column::OrderNumber.eq(&order_num))
                    .one(&state.db)
                    .await
                    .map_err(|e| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(AuthErrorResponse {
                                error: e.to_string(),
                            }),
                        )
                    })?;

                if let Some(ord) = existing {
                    results.push(SyncActionResult {
                        client_action_id: action_id,
                        status: "duplicate_ignored".to_string(),
                        entity_id: Some(ord.id),
                        message: Some("Order already synchronized".to_string()),
                    });
                    continue;
                }

                let subtotal = action
                    .payload
                    .get("subtotal")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Decimal::from_str(s).ok())
                    .unwrap_or(Decimal::ZERO);
                let grand_total = action
                    .payload
                    .get("grand_total")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Decimal::from_str(s).ok())
                    .unwrap_or(subtotal);

                let customer_name = action
                    .payload
                    .get("customer_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let table_or_room = action
                    .payload
                    .get("table_or_room")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let new_order_id = Uuid::new_v4();
                let now = Utc::now();
                let new_order = order::ActiveModel {
                    id: Set(new_order_id),
                    tenant_id: Set(auth.tenant_id.0),
                    order_number: Set(order_num),
                    customer_name: Set(customer_name),
                    table_or_room: Set(table_or_room),
                    status: Set("CONFIRMED".to_string()),
                    subtotal: Set(subtotal),
                    tax_total: Set(Decimal::ZERO),
                    discount_total: Set(Decimal::ZERO),
                    grand_total: Set(grand_total),
                    metadata: Set(action.payload.to_string()),
                    created_at: Set(action.client_timestamp),
                    updated_at: Set(now),
                };

                match new_order.insert(&state.db).await {
                    Ok(saved) => results.push(SyncActionResult {
                        client_action_id: action_id,
                        status: "applied".to_string(),
                        entity_id: Some(saved.id),
                        message: None,
                    }),
                    Err(e) => results.push(SyncActionResult {
                        client_action_id: action_id,
                        status: "failed".to_string(),
                        entity_id: None,
                        message: Some(e.to_string()),
                    }),
                }
            }

            "check_in_booking" => {
                let booking_id = action
                    .payload
                    .get("booking_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok());

                if let Some(b_id) = booking_id {
                    let existing = booking::Entity::find_by_id(b_id)
                        .filter(booking::Column::TenantId.eq(auth.tenant_id.0))
                        .one(&state.db)
                        .await
                        .map_err(|e| {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(AuthErrorResponse {
                                    error: e.to_string(),
                                }),
                            )
                        })?;

                    if let Some(b) = existing {
                        let mut active: booking::ActiveModel = b.into();
                        active.status = Set("CHECKED_IN".to_string());
                        match active.update(&state.db).await {
                            Ok(updated) => results.push(SyncActionResult {
                                client_action_id: action_id,
                                status: "applied".to_string(),
                                entity_id: Some(updated.id),
                                message: None,
                            }),
                            Err(e) => results.push(SyncActionResult {
                                client_action_id: action_id,
                                status: "failed".to_string(),
                                entity_id: None,
                                message: Some(e.to_string()),
                            }),
                        }
                    } else {
                        results.push(SyncActionResult {
                            client_action_id: action_id,
                            status: "failed".to_string(),
                            entity_id: None,
                            message: Some("Booking not found".to_string()),
                        });
                    }
                } else {
                    results.push(SyncActionResult {
                        client_action_id: action_id,
                        status: "failed".to_string(),
                        entity_id: None,
                        message: Some("Missing valid booking_id".to_string()),
                    });
                }
            }

            "record_payment" => {
                let amount = action
                    .payload
                    .get("amount")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Decimal::from_str(s).ok())
                    .unwrap_or(Decimal::ZERO);
                let payment_method = action
                    .payload
                    .get("payment_method")
                    .and_then(|v| v.as_str())
                    .unwrap_or("CASH")
                    .to_string();
                let ref_num = action
                    .payload
                    .get("reference_number")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let payment_id = Uuid::new_v4();
                let new_payment = payment::ActiveModel {
                    id: Set(payment_id),
                    tenant_id: Set(auth.tenant_id.0),
                    order_id: Set(None),
                    customer_id: Set(None),
                    amount: Set(amount),
                    payment_method: Set(payment_method),
                    reference_number: Set(ref_num),
                    status: Set("SUCCESS".to_string()),
                    created_at: Set(action.client_timestamp),
                };

                match new_payment.insert(&state.db).await {
                    Ok(saved) => results.push(SyncActionResult {
                        client_action_id: action_id,
                        status: "applied".to_string(),
                        entity_id: Some(saved.id),
                        message: None,
                    }),
                    Err(e) => results.push(SyncActionResult {
                        client_action_id: action_id,
                        status: "failed".to_string(),
                        entity_id: None,
                        message: Some(e.to_string()),
                    }),
                }
            }

            _ => {
                results.push(SyncActionResult {
                    client_action_id: action_id,
                    status: "failed".to_string(),
                    entity_id: None,
                    message: Some(format!("Unknown action_type: {}", action.action_type)),
                });
            }
        }
    }

    let processed = results.len();
    Ok(Json(SyncPushResponse { processed, results }))
}

#[utoipa::path(
    get,
    path = "/api/v1/sync/pull",
    tag = "Sync",
    security(("bearer_auth" = [])),
    params(SyncPullQuery),
    responses(
        (status = 200, description = "Incremental changes delta retrieved for local cache update", body = SyncPullResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn sync_pull(
    auth: AuthUser,
    State(state): State<SharedState>,
    Query(params): Query<SyncPullQuery>,
) -> Result<Json<SyncPullResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let now = Utc::now();

    let mut prod_query = product::Entity::find()
        .filter(product::Column::TenantId.eq(auth.tenant_id.0))
        .order_by_desc(product::Column::UpdatedAt);

    let mut ord_query = order::Entity::find()
        .filter(order::Column::TenantId.eq(auth.tenant_id.0))
        .order_by_desc(order::Column::UpdatedAt);

    let mut book_query = booking::Entity::find()
        .filter(booking::Column::TenantId.eq(auth.tenant_id.0))
        .order_by_desc(booking::Column::CreatedAt);

    if let Some(since) = params.since {
        prod_query = prod_query.filter(product::Column::UpdatedAt.gte(since));
        ord_query = ord_query.filter(order::Column::UpdatedAt.gte(since));
        book_query = book_query.filter(booking::Column::CreatedAt.gte(since));
    }

    let products = prod_query
        .limit(200)
        .all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error fetching sync products: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: "A database error occurred".to_string(),
                }),
            )
        })?
        .into_iter()
        .map(|p| serde_json::to_value(p).unwrap_or_default())
        .collect::<Vec<_>>();

    let orders = ord_query
        .limit(200)
        .all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error fetching sync orders: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: "A database error occurred".to_string(),
                }),
            )
        })?
        .into_iter()
        .map(|o| serde_json::to_value(o).unwrap_or_default())
        .collect::<Vec<_>>();

    let bookings = book_query
        .limit(200)
        .all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error fetching sync bookings: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: "A database error occurred".to_string(),
                }),
            )
        })?
        .into_iter()
        .map(|b| serde_json::to_value(b).unwrap_or_default())
        .collect::<Vec<_>>();

    Ok(Json(SyncPullResponse {
        sync_cursor: now,
        products_count: products.len(),
        orders_count: orders.len(),
        bookings_count: bookings.len(),
        products,
        orders,
        bookings,
    }))
}
