use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use obaf_database::entities::stock_movement;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct RecordStockMovementRequest {
    pub product_id: Uuid,
    pub movement_type: String, // "RECEIPT", "SALE", "ADJUSTMENT", "RETURN"
    pub quantity: Decimal,     // positive for receipt, negative for sale
    pub batch_number: Option<String>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub reference_id: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct StockMovementResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub product_id: Uuid,
    pub movement_type: String,
    pub quantity: Decimal,
    pub batch_number: Option<String>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub reference_id: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<stock_movement::Model> for StockMovementResponse {
    fn from(m: stock_movement::Model) -> Self {
        Self {
            id: m.id,
            tenant_id: m.tenant_id,
            product_id: m.product_id,
            movement_type: m.movement_type,
            quantity: m.quantity,
            batch_number: m.batch_number,
            expiry_date: m.expiry_date,
            reference_id: m.reference_id,
            notes: m.notes,
            created_at: m.created_at,
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/inventory/movements",
    tag = "Inventory",
    security(("bearer_auth" = [])),
    request_body = RecordStockMovementRequest,
    responses(
        (status = 201, description = "Stock movement recorded successfully", body = StockMovementResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn record_movement(
    State(state): State<SharedState>,
    auth: AuthUser,
    Json(payload): Json<RecordStockMovementRequest>,
) -> Result<(StatusCode, Json<StockMovementResponse>), (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager", "staff"])?;

    let movement_id = Uuid::new_v4();
    let now = Utc::now();

    let new_movement = stock_movement::ActiveModel {
        id: Set(movement_id),
        tenant_id: Set(auth.tenant_id.as_uuid()),
        product_id: Set(payload.product_id),
        movement_type: Set(payload.movement_type),
        quantity: Set(payload.quantity),
        batch_number: Set(payload.batch_number),
        expiry_date: Set(payload.expiry_date),
        reference_id: Set(payload.reference_id),
        notes: Set(payload.notes),
        created_at: Set(now),
    };

    let inserted = new_movement.insert(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to record stock movement: {}", e),
            }),
        )
    })?;

    Ok((StatusCode::CREATED, Json(inserted.into())))
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct InventoryFilterQuery {
    pub product_id: Option<Uuid>,
    pub movement_type: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/inventory/movements",
    tag = "Inventory",
    security(("bearer_auth" = [])),
    params(
        InventoryFilterQuery
    ),
    responses(
        (status = 200, description = "List of stock movements for current tenant", body = Vec<StockMovementResponse>),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn list_movements(
    State(state): State<SharedState>,
    auth: AuthUser,
    Query(query): Query<InventoryFilterQuery>,
) -> Result<Json<Vec<StockMovementResponse>>, (StatusCode, Json<AuthErrorResponse>)> {
    let mut q = stock_movement::Entity::find()
        .filter(stock_movement::Column::TenantId.eq(auth.tenant_id.as_uuid()));

    if let Some(pid) = query.product_id {
        q = q.filter(stock_movement::Column::ProductId.eq(pid));
    }

    if let Some(m_type) = query.movement_type {
        q = q.filter(stock_movement::Column::MovementType.eq(m_type));
    }

    if let Some(limit) = query.limit {
        q = q.limit(limit);
    }

    if let Some(offset) = query.offset {
        q = q.offset(offset);
    }

    let movements = q.all(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Database error: {}", e),
            }),
        )
    })?;

    let resp = movements.into_iter().map(Into::into).collect();
    Ok(Json(resp))
}

#[derive(Serialize, ToSchema)]
pub struct ProductStockBalance {
    pub product_id: Uuid,
    pub on_hand_quantity: Decimal,
    pub movements_count: usize,
}

#[utoipa::path(
    get,
    path = "/api/v1/inventory/balance/{product_id}",
    tag = "Inventory",
    security(("bearer_auth" = [])),
    params(
        ("product_id" = Uuid, Path, description = "Product ID to calculate ledger balance for")
    ),
    responses(
        (status = 200, description = "Current net on-hand stock balance calculated from ledger", body = ProductStockBalance),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn get_stock_balance(
    State(state): State<SharedState>,
    auth: AuthUser,
    axum::extract::Path(product_id): axum::extract::Path<Uuid>,
) -> Result<Json<ProductStockBalance>, (StatusCode, Json<AuthErrorResponse>)> {
    let movements = stock_movement::Entity::find()
        .filter(stock_movement::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .filter(stock_movement::Column::ProductId.eq(product_id))
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

    let on_hand_quantity = movements.iter().map(|m| m.quantity).sum();
    let movements_count = movements.len();

    Ok(Json(ProductStockBalance {
        product_id,
        on_hand_quantity,
        movements_count,
    }))
}

#[derive(Deserialize, ToSchema)]
pub struct CreateStockLocationRequest {
    pub name: String,
    pub code: String,
    pub address: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct StockLocationResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub code: String,
    pub address: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<obaf_database::entities::stock_location::Model> for StockLocationResponse {
    fn from(m: obaf_database::entities::stock_location::Model) -> Self {
        Self {
            id: m.id,
            tenant_id: m.tenant_id,
            name: m.name,
            code: m.code,
            address: m.address,
            is_active: m.is_active,
            created_at: m.created_at,
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/inventory/locations",
    tag = "Inventory",
    security(("bearer_auth" = [])),
    request_body = CreateStockLocationRequest,
    responses(
        (status = 201, description = "Stock location created successfully", body = StockLocationResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn create_stock_location(
    State(state): State<SharedState>,
    auth: AuthUser,
    Json(payload): Json<CreateStockLocationRequest>,
) -> Result<(StatusCode, Json<StockLocationResponse>), (StatusCode, Json<AuthErrorResponse>)> {
    use obaf_database::entities::stock_location;

    let loc_id = Uuid::new_v4();
    let now = Utc::now();

    let new_loc = stock_location::ActiveModel {
        id: Set(loc_id),
        tenant_id: Set(auth.tenant_id.as_uuid()),
        name: Set(payload.name),
        code: Set(payload.code.to_uppercase()),
        address: Set(payload.address),
        is_active: Set(true),
        created_at: Set(now),
    };

    let inserted = new_loc.insert(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to create stock location: {}", e),
            }),
        )
    })?;

    Ok((StatusCode::CREATED, Json(inserted.into())))
}

#[utoipa::path(
    get,
    path = "/api/v1/inventory/locations",
    tag = "Inventory",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of stock locations and warehouses", body = Vec<StockLocationResponse>),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn list_stock_locations(
    State(state): State<SharedState>,
    auth: AuthUser,
) -> Result<Json<Vec<StockLocationResponse>>, (StatusCode, Json<AuthErrorResponse>)> {
    use obaf_database::entities::stock_location;

    let locations = stock_location::Entity::find()
        .filter(stock_location::Column::TenantId.eq(auth.tenant_id.as_uuid()))
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

    Ok(Json(locations.into_iter().map(Into::into).collect()))
}
