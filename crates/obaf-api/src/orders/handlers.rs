use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use obaf_database::entities::{order, order_item, product};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct CreateOrderItemRequest {
    pub product_id: Uuid,
    pub quantity: Decimal,
    pub notes: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateOrderRequest {
    pub customer_name: Option<String>,
    pub table_or_room: Option<String>,
    pub items: Vec<CreateOrderItemRequest>,
}

#[derive(Deserialize, IntoParams)]
pub struct OrderFilterQuery {
    pub status: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct OrderItemResponse {
    pub id: Uuid,
    pub product_id: Uuid,
    pub item_name: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub tax_rate: Decimal,
    pub subtotal: Decimal,
    pub notes: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct OrderResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub order_number: String,
    pub customer_name: Option<String>,
    pub table_or_room: Option<String>,
    pub status: String,
    pub subtotal: Decimal,
    pub tax_total: Decimal,
    pub discount_total: Decimal,
    pub grand_total: Decimal,
    pub items: Vec<OrderItemResponse>,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateOrderStatusRequest {
    pub status: String, // "CONFIRMED", "IN_PROGRESS", "READY", "COMPLETED", "CANCELLED"
}

#[utoipa::path(
    post,
    path = "/api/v1/orders",
    tag = "Orders",
    security(("bearer_auth" = [])),
    request_body = CreateOrderRequest,
    responses(
        (status = 201, description = "Order created successfully", body = OrderResponse),
        (status = 400, description = "Invalid request or empty items", body = AuthErrorResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn create_order(
    State(state): State<SharedState>,
    auth: AuthUser,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<OrderResponse>), (StatusCode, Json<AuthErrorResponse>)> {
    if payload.items.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: "Order must contain at least one item".to_string(),
            }),
        ));
    }

    let txn = state.db.begin().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Database transaction failed: {}", e),
            }),
        )
    })?;

    let order_id = Uuid::new_v4();
    let now = Utc::now();
    let order_num = format!("ORD-{}", &order_id.to_string()[..8].to_uppercase());

    let mut subtotal = Decimal::ZERO;
    let mut tax_total = Decimal::ZERO;
    let mut item_responses = Vec::new();
    let mut items_to_insert = Vec::new();

    for item_req in &payload.items {
        let p = product::Entity::find()
            .filter(product::Column::Id.eq(item_req.product_id))
            .filter(product::Column::TenantId.eq(auth.tenant_id.as_uuid()))
            .one(&txn)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(AuthErrorResponse {
                        error: format!("Failed to fetch product: {}", e),
                    }),
                )
            })?
            .ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(AuthErrorResponse {
                        error: format!("Product '{}' not found", item_req.product_id),
                    }),
                )
            })?;

        let line_subtotal = p.price * item_req.quantity;
        let line_tax = line_subtotal * p.tax_rate;
        subtotal += line_subtotal;
        tax_total += line_tax;

        let item_id = Uuid::new_v4();
        let active_item = order_item::ActiveModel {
            id: Set(item_id),
            order_id: Set(order_id),
            product_id: Set(p.id),
            item_name: Set(p.name.clone()),
            quantity: Set(item_req.quantity),
            unit_price: Set(p.price),
            tax_rate: Set(p.tax_rate),
            subtotal: Set(line_subtotal),
            notes: Set(item_req.notes.clone()),
        };

        items_to_insert.push(active_item);

        item_responses.push(OrderItemResponse {
            id: item_id,
            product_id: p.id,
            item_name: p.name,
            quantity: item_req.quantity,
            unit_price: p.price,
            tax_rate: p.tax_rate,
            subtotal: line_subtotal,
            notes: item_req.notes.clone(),
        });
    }

    let grand_total = subtotal + tax_total;

    // 1. Insert parent order record first
    let active_order = order::ActiveModel {
        id: Set(order_id),
        tenant_id: Set(auth.tenant_id.as_uuid()),
        order_number: Set(order_num.clone()),
        customer_name: Set(payload.customer_name.clone()),
        table_or_room: Set(payload.table_or_room.clone()),
        status: Set("CONFIRMED".to_string()),
        subtotal: Set(subtotal),
        tax_total: Set(tax_total),
        discount_total: Set(Decimal::ZERO),
        grand_total: Set(grand_total),
        metadata: Set("{}".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };

    active_order.insert(&txn).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to create order: {}", e),
            }),
        )
    })?;

    // 2. Insert child order items referencing parent order_id
    for item in items_to_insert {
        item.insert(&txn).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Failed to insert order item: {}", e),
                }),
            )
        })?;
    }

    txn.commit().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to commit order: {}", e),
            }),
        )
    })?;

    // Real-time broadcast for Kitchen Display System (KDS) & POS screens
    state.broadcast_event(
        "order.created",
        serde_json::json!({
            "order_id": order_id,
            "order_number": order_num,
            "table_or_room": payload.table_or_room,
            "grand_total": grand_total,
            "status": "CONFIRMED"
        }),
    );

    Ok((
        StatusCode::CREATED,
        Json(OrderResponse {
            id: order_id,
            tenant_id: auth.tenant_id.as_uuid(),
            order_number: order_num,
            customer_name: payload.customer_name,
            table_or_room: payload.table_or_room,
            status: "CONFIRMED".to_string(),
            subtotal,
            tax_total,
            discount_total: Decimal::ZERO,
            grand_total,
            items: item_responses,
            created_at: now,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/orders",
    tag = "Orders",
    security(("bearer_auth" = [])),
    params(
        OrderFilterQuery
    ),
    responses(
        (status = 200, description = "List of orders", body = Vec<OrderResponse>),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn list_orders(
    State(state): State<SharedState>,
    auth: AuthUser,
    Query(query): Query<OrderFilterQuery>,
) -> Result<Json<Vec<OrderResponse>>, (StatusCode, Json<AuthErrorResponse>)> {
    let mut q = order::Entity::find().filter(order::Column::TenantId.eq(auth.tenant_id.as_uuid()));

    if let Some(status) = query.status {
        q = q.filter(order::Column::Status.eq(status));
    }

    let orders = q.all(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Database error: {}", e),
            }),
        )
    })?;

    let mut resp = Vec::new();
    for o in orders {
        let items = order_item::Entity::find()
            .filter(order_item::Column::OrderId.eq(o.id))
            .all(&state.db)
            .await
            .unwrap_or_default();

        let item_resps = items
            .into_iter()
            .map(|i| OrderItemResponse {
                id: i.id,
                product_id: i.product_id,
                item_name: i.item_name,
                quantity: i.quantity,
                unit_price: i.unit_price,
                tax_rate: i.tax_rate,
                subtotal: i.subtotal,
                notes: i.notes,
            })
            .collect();

        resp.push(OrderResponse {
            id: o.id,
            tenant_id: o.tenant_id,
            order_number: o.order_number,
            customer_name: o.customer_name,
            table_or_room: o.table_or_room,
            status: o.status,
            subtotal: o.subtotal,
            tax_total: o.tax_total,
            discount_total: o.discount_total,
            grand_total: o.grand_total,
            items: item_resps,
            created_at: o.created_at,
        });
    }

    Ok(Json(resp))
}

#[utoipa::path(
    patch,
    path = "/api/v1/orders/{id}/status",
    tag = "Orders",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Order ID")
    ),
    request_body = UpdateOrderStatusRequest,
    responses(
        (status = 200, description = "Order status updated successfully", body = OrderResponse),
        (status = 404, description = "Order not found", body = AuthErrorResponse)
    )
)]
pub async fn update_status(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateOrderStatusRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<AuthErrorResponse>)> {
    let order_model = order::Entity::find()
        .filter(order::Column::Id.eq(id))
        .filter(order::Column::TenantId.eq(auth.tenant_id.as_uuid()))
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
                    error: "Order not found".to_string(),
                }),
            )
        })?;

    let mut active: order::ActiveModel = order_model.into();
    active.status = Set(payload.status.clone());
    active.updated_at = Set(Utc::now());

    active.update(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to update order status: {}", e),
            }),
        )
    })?;

    // Real-time broadcast for live kitchen & status updates
    state.broadcast_event(
        "order.status_updated",
        serde_json::json!({
            "order_id": id,
            "new_status": payload.status
        }),
    );

    Ok(Json(serde_json::json!({
        "order_id": id,
        "new_status": payload.status,
        "message": "Order status updated successfully"
    })))
}
