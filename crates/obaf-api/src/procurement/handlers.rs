use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use obaf_database::entities::{
    product, purchase_order, purchase_order_item, stock_movement, supplier,
};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct CreatePurchaseOrderItemRequest {
    pub product_id: Uuid,
    pub item_name: String,
    pub quantity_ordered: Decimal,
    pub unit_cost: Decimal,
}

#[derive(Deserialize, ToSchema)]
pub struct CreatePurchaseOrderRequest {
    pub supplier_id: Uuid,
    pub items: Vec<CreatePurchaseOrderItemRequest>,
    pub expected_delivery_date: Option<DateTime<Utc>>,
    pub currency: Option<String>,
    pub tax_rate: Option<Decimal>,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PurchaseOrderItemResponse {
    pub id: Uuid,
    pub purchase_order_id: Uuid,
    pub product_id: Uuid,
    pub item_name: String,
    pub quantity_ordered: Decimal,
    pub quantity_received: Decimal,
    pub unit_cost: Decimal,
    pub subtotal: Decimal,
}

impl From<purchase_order_item::Model> for PurchaseOrderItemResponse {
    fn from(m: purchase_order_item::Model) -> Self {
        Self {
            id: m.id,
            purchase_order_id: m.purchase_order_id,
            product_id: m.product_id,
            item_name: m.item_name,
            quantity_ordered: m.quantity_ordered,
            quantity_received: m.quantity_received,
            unit_cost: m.unit_cost,
            subtotal: m.subtotal,
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PurchaseOrderDetailResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub po_number: String,
    pub supplier_id: Uuid,
    pub status: String,
    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub total_amount: Decimal,
    pub currency: String,
    pub notes: Option<String>,
    pub expected_delivery_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub items: Vec<PurchaseOrderItemResponse>,
}

#[derive(Deserialize, IntoParams)]
pub struct ListPurchaseOrdersQuery {
    pub status: Option<String>,
    pub supplier_id: Option<Uuid>,
}

#[derive(Deserialize, ToSchema)]
pub struct ReceiveItemInput {
    pub item_id: Uuid,
    pub quantity_received: Decimal,
    pub batch_number: Option<String>,
    pub expiry_date: Option<DateTime<Utc>>,
}

#[derive(Deserialize, ToSchema)]
pub struct ReceivePurchaseOrderRequest {
    pub items: Vec<ReceiveItemInput>,
}

#[utoipa::path(
    post,
    path = "/api/v1/procurement/purchase-orders",
    tag = "Procurement",
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
    request_body = CreatePurchaseOrderRequest,
    responses(
        (status = 201, description = "Purchase order created successfully", body = PurchaseOrderDetailResponse),
        (status = 400, description = "Validation error", body = AuthErrorResponse),
        (status = 404, description = "Supplier or Product not found", body = AuthErrorResponse)
    )
)]
pub async fn create_purchase_order(
    State(state): State<SharedState>,
    auth_user: AuthUser,
    Json(payload): Json<CreatePurchaseOrderRequest>,
) -> Result<(StatusCode, Json<PurchaseOrderDetailResponse>), (StatusCode, Json<AuthErrorResponse>)>
{
    let tenant_id = auth_user.tenant_id.0;

    if payload.items.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: "Purchase order must contain at least one line item".to_string(),
            }),
        ));
    }

    // Verify supplier exists for tenant
    let supp = supplier::Entity::find_by_id(payload.supplier_id)
        .filter(supplier::Column::TenantId.eq(tenant_id))
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

    if supp.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(AuthErrorResponse {
                error: "Supplier not found for current tenant".to_string(),
            }),
        ));
    }

    // Verify products exist and calculate subtotals
    let mut calculated_subtotal = Decimal::ZERO;
    let mut item_records = Vec::new();

    for it in &payload.items {
        if it.quantity_ordered <= Decimal::ZERO {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(AuthErrorResponse {
                    error: format!("Invalid ordered quantity for item: {}", it.item_name),
                }),
            ));
        }

        let prod = product::Entity::find_by_id(it.product_id)
            .filter(product::Column::TenantId.eq(tenant_id))
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

        if prod.is_none() {
            return Err((
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: format!("Product not found: {}", it.product_id),
                }),
            ));
        }

        let line_subtotal = (it.quantity_ordered * it.unit_cost).round_dp(2);
        calculated_subtotal += line_subtotal;
        item_records.push((it, line_subtotal));
    }

    let tax_rate = payload.tax_rate.unwrap_or(Decimal::ZERO);
    let tax_amount = (calculated_subtotal * tax_rate).round_dp(2);
    let total_amount = calculated_subtotal + tax_amount;
    let now = Utc::now();
    let po_id = Uuid::new_v4();
    let po_num = format!(
        "PO-{}-{}",
        now.format("%Y%m%d"),
        Uuid::new_v4().to_string()[..6].to_uppercase()
    );

    let po_model = purchase_order::ActiveModel {
        id: Set(po_id),
        tenant_id: Set(tenant_id),
        po_number: Set(po_num.clone()),
        supplier_id: Set(payload.supplier_id),
        status: Set("SUBMITTED".to_string()),
        subtotal: Set(calculated_subtotal),
        tax_amount: Set(tax_amount),
        total_amount: Set(total_amount),
        currency: Set(payload.currency.unwrap_or_else(|| "USD".to_string())),
        notes: Set(payload.notes),
        expected_delivery_date: Set(payload.expected_delivery_date),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let mut response_items = Vec::new();
    for (it, line_subtotal) in item_records {
        let item_id = Uuid::new_v4();
        let inserted_item = purchase_order_item::ActiveModel {
            id: Set(item_id),
            purchase_order_id: Set(po_id),
            product_id: Set(it.product_id),
            item_name: Set(it.item_name.clone()),
            quantity_ordered: Set(it.quantity_ordered),
            quantity_received: Set(Decimal::ZERO),
            unit_cost: Set(it.unit_cost),
            subtotal: Set(line_subtotal),
        }
        .insert(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

        response_items.push(PurchaseOrderItemResponse::from(inserted_item));
    }

    Ok((
        StatusCode::CREATED,
        Json(PurchaseOrderDetailResponse {
            id: po_model.id,
            tenant_id: po_model.tenant_id,
            po_number: po_model.po_number,
            supplier_id: po_model.supplier_id,
            status: po_model.status,
            subtotal: po_model.subtotal,
            tax_amount: po_model.tax_amount,
            total_amount: po_model.total_amount,
            currency: po_model.currency,
            notes: po_model.notes,
            expected_delivery_date: po_model.expected_delivery_date,
            created_at: po_model.created_at,
            updated_at: po_model.updated_at,
            items: response_items,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/procurement/purchase-orders",
    tag = "Procurement",
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
    params(ListPurchaseOrdersQuery),
    responses(
        (status = 200, description = "List of purchase orders", body = Vec<PurchaseOrderDetailResponse>)
    )
)]
pub async fn list_purchase_orders(
    State(state): State<SharedState>,
    auth_user: AuthUser,
    Query(params): Query<ListPurchaseOrdersQuery>,
) -> Result<Json<Vec<PurchaseOrderDetailResponse>>, (StatusCode, Json<AuthErrorResponse>)> {
    let tenant_id = auth_user.tenant_id.0;
    let mut query = purchase_order::Entity::find()
        .filter(purchase_order::Column::TenantId.eq(tenant_id))
        .order_by_desc(purchase_order::Column::CreatedAt);

    if let Some(status) = params.status {
        query = query.filter(purchase_order::Column::Status.eq(status));
    }

    if let Some(supplier_id) = params.supplier_id {
        query = query.filter(purchase_order::Column::SupplierId.eq(supplier_id));
    }

    let pos = query.all(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let mut results = Vec::new();
    for po in pos {
        let items = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::PurchaseOrderId.eq(po.id))
            .all(&state.db)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(AuthErrorResponse {
                        error: e.to_string(),
                    }),
                )
            })?;

        results.push(PurchaseOrderDetailResponse {
            id: po.id,
            tenant_id: po.tenant_id,
            po_number: po.po_number,
            supplier_id: po.supplier_id,
            status: po.status,
            subtotal: po.subtotal,
            tax_amount: po.tax_amount,
            total_amount: po.total_amount,
            currency: po.currency,
            notes: po.notes,
            expected_delivery_date: po.expected_delivery_date,
            created_at: po.created_at,
            updated_at: po.updated_at,
            items: items
                .into_iter()
                .map(PurchaseOrderItemResponse::from)
                .collect(),
        });
    }

    Ok(Json(results))
}

#[utoipa::path(
    get,
    path = "/api/v1/procurement/purchase-orders/{id}",
    tag = "Procurement",
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
    params(
        ("id" = Uuid, Path, description = "Purchase Order ID")
    ),
    responses(
        (status = 200, description = "Purchase order details", body = PurchaseOrderDetailResponse),
        (status = 404, description = "Purchase order not found", body = AuthErrorResponse)
    )
)]
pub async fn get_purchase_order(
    State(state): State<SharedState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<PurchaseOrderDetailResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let tenant_id = auth_user.tenant_id.0;
    let po = purchase_order::Entity::find_by_id(id)
        .filter(purchase_order::Column::TenantId.eq(tenant_id))
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

    match po {
        Some(po) => {
            let items = purchase_order_item::Entity::find()
                .filter(purchase_order_item::Column::PurchaseOrderId.eq(po.id))
                .all(&state.db)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(AuthErrorResponse {
                            error: e.to_string(),
                        }),
                    )
                })?;

            Ok(Json(PurchaseOrderDetailResponse {
                id: po.id,
                tenant_id: po.tenant_id,
                po_number: po.po_number,
                supplier_id: po.supplier_id,
                status: po.status,
                subtotal: po.subtotal,
                tax_amount: po.tax_amount,
                total_amount: po.total_amount,
                currency: po.currency,
                notes: po.notes,
                expected_delivery_date: po.expected_delivery_date,
                created_at: po.created_at,
                updated_at: po.updated_at,
                items: items
                    .into_iter()
                    .map(PurchaseOrderItemResponse::from)
                    .collect(),
            }))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(AuthErrorResponse {
                error: "Purchase order not found".to_string(),
            }),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/procurement/purchase-orders/{id}/receive",
    tag = "Procurement",
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
    params(
        ("id" = Uuid, Path, description = "Purchase Order ID")
    ),
    request_body = ReceivePurchaseOrderRequest,
    responses(
        (status = 200, description = "Items received and stock movements recorded", body = PurchaseOrderDetailResponse),
        (status = 400, description = "Cannot receive cancelled or already completed PO", body = AuthErrorResponse),
        (status = 404, description = "Purchase order not found", body = AuthErrorResponse)
    )
)]
pub async fn receive_purchase_order(
    State(state): State<SharedState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<ReceivePurchaseOrderRequest>,
) -> Result<Json<PurchaseOrderDetailResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let tenant_id = auth_user.tenant_id.0;

    let po = purchase_order::Entity::find_by_id(id)
        .filter(purchase_order::Column::TenantId.eq(tenant_id))
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

    let po = match po {
        Some(p) => p,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: "Purchase order not found".to_string(),
                }),
            ))
        }
    };

    if po.status == "CANCELLED" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: "Cannot receive goods against a cancelled purchase order".to_string(),
            }),
        ));
    }

    let now = Utc::now();

    // Process each item receive input
    for rec in payload.items {
        if rec.quantity_received <= Decimal::ZERO {
            continue;
        }

        let item = purchase_order_item::Entity::find_by_id(rec.item_id)
            .filter(purchase_order_item::Column::PurchaseOrderId.eq(po.id))
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

        if let Some(item_model) = item {
            // Update quantity_received on line item
            let updated_qty = item_model.quantity_received + rec.quantity_received;
            let mut active_item: purchase_order_item::ActiveModel = item_model.clone().into();
            active_item.quantity_received = Set(updated_qty);
            active_item.update(&state.db).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(AuthErrorResponse {
                        error: e.to_string(),
                    }),
                )
            })?;

            // Record inbound stock movement
            stock_movement::ActiveModel {
                id: Set(Uuid::new_v4()),
                tenant_id: Set(tenant_id),
                product_id: Set(item_model.product_id),
                movement_type: Set("PURCHASE".to_string()),
                quantity: Set(rec.quantity_received),
                batch_number: Set(rec.batch_number),
                expiry_date: Set(rec.expiry_date),
                reference_id: Set(Some(po.po_number.clone())),
                notes: Set(Some(format!(
                    "Inbound receipt from Purchase Order {}",
                    po.po_number
                ))),
                created_at: Set(now),
            }
            .insert(&state.db)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(AuthErrorResponse {
                        error: e.to_string(),
                    }),
                )
            })?;
        }
    }

    // Refresh all items to determine updated overall status
    let refreshed_items = purchase_order_item::Entity::find()
        .filter(purchase_order_item::Column::PurchaseOrderId.eq(po.id))
        .all(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    let all_received = refreshed_items
        .iter()
        .all(|it| it.quantity_received >= it.quantity_ordered);
    let any_received = refreshed_items
        .iter()
        .any(|it| it.quantity_received > Decimal::ZERO);

    let new_status = if all_received {
        "RECEIVED".to_string()
    } else if any_received {
        "PARTIALLY_RECEIVED".to_string()
    } else {
        po.status.clone()
    };

    let mut active_po: purchase_order::ActiveModel = po.into();
    active_po.status = Set(new_status.clone());
    active_po.updated_at = Set(now);
    let updated_po = active_po.update(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(PurchaseOrderDetailResponse {
        id: updated_po.id,
        tenant_id: updated_po.tenant_id,
        po_number: updated_po.po_number,
        supplier_id: updated_po.supplier_id,
        status: updated_po.status,
        subtotal: updated_po.subtotal,
        tax_amount: updated_po.tax_amount,
        total_amount: updated_po.total_amount,
        currency: updated_po.currency,
        notes: updated_po.notes,
        expected_delivery_date: updated_po.expected_delivery_date,
        created_at: updated_po.created_at,
        updated_at: updated_po.updated_at,
        items: refreshed_items
            .into_iter()
            .map(PurchaseOrderItemResponse::from)
            .collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/procurement/purchase-orders/{id}/cancel",
    tag = "Procurement",
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
    params(
        ("id" = Uuid, Path, description = "Purchase Order ID")
    ),
    responses(
        (status = 200, description = "Purchase order cancelled", body = PurchaseOrderDetailResponse),
        (status = 400, description = "Cannot cancel received or partially received PO", body = AuthErrorResponse),
        (status = 404, description = "Purchase order not found", body = AuthErrorResponse)
    )
)]
pub async fn cancel_purchase_order(
    State(state): State<SharedState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<PurchaseOrderDetailResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let tenant_id = auth_user.tenant_id.0;

    let po = purchase_order::Entity::find_by_id(id)
        .filter(purchase_order::Column::TenantId.eq(tenant_id))
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

    let po = match po {
        Some(p) => p,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: "Purchase order not found".to_string(),
                }),
            ))
        }
    };

    if po.status == "RECEIVED" || po.status == "PARTIALLY_RECEIVED" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: "Cannot cancel a purchase order that has already received inventory"
                    .to_string(),
            }),
        ));
    }

    let now = Utc::now();
    let mut active_po: purchase_order::ActiveModel = po.into();
    active_po.status = Set("CANCELLED".to_string());
    active_po.updated_at = Set(now);
    let updated_po = active_po.update(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let items = purchase_order_item::Entity::find()
        .filter(purchase_order_item::Column::PurchaseOrderId.eq(updated_po.id))
        .all(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(Json(PurchaseOrderDetailResponse {
        id: updated_po.id,
        tenant_id: updated_po.tenant_id,
        po_number: updated_po.po_number,
        supplier_id: updated_po.supplier_id,
        status: updated_po.status,
        subtotal: updated_po.subtotal,
        tax_amount: updated_po.tax_amount,
        total_amount: updated_po.total_amount,
        currency: updated_po.currency,
        notes: updated_po.notes,
        expected_delivery_date: updated_po.expected_delivery_date,
        created_at: updated_po.created_at,
        updated_at: updated_po.updated_at,
        items: items
            .into_iter()
            .map(PurchaseOrderItemResponse::from)
            .collect(),
    }))
}
