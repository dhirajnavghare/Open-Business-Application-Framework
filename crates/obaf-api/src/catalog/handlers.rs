use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use obaf_database::entities::product;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct CreateProductRequest {
    pub sku: String,
    pub name: String,
    pub price: Decimal,
    pub cost_price: Decimal,
    pub tax_rate: Decimal, // e.g. 0.05, 0.18
    pub category_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>, // Dynamic industry attributes (dosage, spicy level, etc.)
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateProductRequest {
    pub sku: Option<String>,
    pub name: Option<String>,
    pub price: Option<Decimal>,
    pub cost_price: Option<Decimal>,
    pub tax_rate: Option<Decimal>,
    pub category_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
    pub is_active: Option<bool>,
}

#[derive(Serialize, ToSchema)]
pub struct ProductResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub sku: String,
    pub name: String,
    pub price: Decimal,
    pub cost_price: Decimal,
    pub tax_rate: Decimal,
    pub metadata: serde_json::Value,
    pub is_active: bool,
}

impl From<product::Model> for ProductResponse {
    fn from(m: product::Model) -> Self {
        let meta = serde_json::from_str(&m.metadata).unwrap_or(serde_json::json!({}));
        Self {
            id: m.id,
            tenant_id: m.tenant_id,
            sku: m.sku,
            name: m.name,
            price: m.price,
            cost_price: m.cost_price,
            tax_rate: m.tax_rate,
            metadata: meta,
            is_active: m.is_active,
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/catalog/products",
    tag = "Catalog",
    security(("bearer_auth" = [])),
    request_body = CreateProductRequest,
    responses(
        (status = 201, description = "Product created successfully", body = ProductResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn create_product(
    State(state): State<SharedState>,
    auth: AuthUser,
    Json(payload): Json<CreateProductRequest>,
) -> Result<(StatusCode, Json<ProductResponse>), (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager"])?;

    if payload.price < Decimal::ZERO {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: "Product price cannot be negative".to_string(),
            }),
        ));
    }
    if payload.cost_price < Decimal::ZERO {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: "Product cost price cannot be negative".to_string(),
            }),
        ));
    }
    if payload.tax_rate < Decimal::ZERO {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: "Product tax rate cannot be negative".to_string(),
            }),
        ));
    }

    let product_id = Uuid::new_v4();
    let now = Utc::now();
    let meta_str = payload
        .metadata
        .map(|v| v.to_string())
        .unwrap_or_else(|| "{}".to_string());

    let new_product = product::ActiveModel {
        id: Set(product_id),
        tenant_id: Set(auth.tenant_id.as_uuid()),
        category_id: Set(payload.category_id),
        sku: Set(payload.sku),
        name: Set(payload.name),
        price: Set(payload.price),
        cost_price: Set(payload.cost_price),
        tax_rate: Set(payload.tax_rate),
        metadata: Set(meta_str),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let inserted = new_product.insert(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to create product: {}", e),
            }),
        )
    })?;

    Ok((StatusCode::CREATED, Json(inserted.into())))
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct ProductFilterQuery {
    pub search: Option<String>,
    pub category_id: Option<Uuid>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/catalog/products",
    tag = "Catalog",
    security(("bearer_auth" = [])),
    params(
        ProductFilterQuery
    ),
    responses(
        (status = 200, description = "List of products for current tenant", body = Vec<ProductResponse>),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn list_products(
    State(state): State<SharedState>,
    auth: AuthUser,
    Query(query): Query<ProductFilterQuery>,
) -> Result<Json<Vec<ProductResponse>>, (StatusCode, Json<AuthErrorResponse>)> {
    let mut q =
        product::Entity::find().filter(product::Column::TenantId.eq(auth.tenant_id.as_uuid()));

    if let Some(cat) = query.category_id {
        q = q.filter(product::Column::CategoryId.eq(cat));
    }

    if let Some(s) = query.search {
        let pattern = format!("%{}%", s);
        q = q.filter(
            product::Column::Name
                .like(&pattern)
                .or(product::Column::Sku.like(&pattern)),
        );
    }

    if let Some(limit) = query.limit {
        q = q.limit(limit);
    }

    if let Some(offset) = query.offset {
        q = q.offset(offset);
    }

    let products = q.all(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Database error: {}", e),
            }),
        )
    })?;

    let resp = products.into_iter().map(Into::into).collect();
    Ok(Json(resp))
}

#[utoipa::path(
    get,
    path = "/api/v1/catalog/products/{id}",
    tag = "Catalog",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Product ID")
    ),
    responses(
        (status = 200, description = "Product details", body = ProductResponse),
        (status = 404, description = "Product not found", body = AuthErrorResponse)
    )
)]
pub async fn get_product(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ProductResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let p = product::Entity::find()
        .filter(product::Column::Id.eq(id))
        .filter(product::Column::TenantId.eq(auth.tenant_id.as_uuid()))
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
                    error: "Product not found".to_string(),
                }),
            )
        })?;

    Ok(Json(p.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/catalog/products/{id}",
    tag = "Catalog",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Product ID")
    ),
    request_body = UpdateProductRequest,
    responses(
        (status = 200, description = "Product updated successfully", body = ProductResponse),
        (status = 404, description = "Product not found", body = AuthErrorResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn update_product(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateProductRequest>,
) -> Result<Json<ProductResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager"])?;

    let p = product::Entity::find()
        .filter(product::Column::Id.eq(id))
        .filter(product::Column::TenantId.eq(auth.tenant_id.as_uuid()))
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
                    error: "Product not found".to_string(),
                }),
            )
        })?;

    let mut active: product::ActiveModel = p.into();

    if let Some(sku) = payload.sku {
        active.sku = Set(sku);
    }
    if let Some(name) = payload.name {
        active.name = Set(name);
    }
    if let Some(price) = payload.price {
        if price < Decimal::ZERO {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(AuthErrorResponse {
                    error: "Product price cannot be negative".to_string(),
                }),
            ));
        }
        active.price = Set(price);
    }
    if let Some(cost_price) = payload.cost_price {
        if cost_price < Decimal::ZERO {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(AuthErrorResponse {
                    error: "Product cost price cannot be negative".to_string(),
                }),
            ));
        }
        active.cost_price = Set(cost_price);
    }
    if let Some(tax_rate) = payload.tax_rate {
        if tax_rate < Decimal::ZERO {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(AuthErrorResponse {
                    error: "Product tax rate cannot be negative".to_string(),
                }),
            ));
        }
        active.tax_rate = Set(tax_rate);
    }
    if let Some(cat_id) = payload.category_id {
        active.category_id = Set(Some(cat_id));
    }
    if let Some(meta) = payload.metadata {
        active.metadata = Set(meta.to_string());
    }
    if let Some(is_active) = payload.is_active {
        active.is_active = Set(is_active);
    }
    active.updated_at = Set(Utc::now());

    let updated = active.update(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to update product: {}", e),
            }),
        )
    })?;

    Ok(Json(updated.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/catalog/products/{id}",
    tag = "Catalog",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Product ID")
    ),
    responses(
        (status = 200, description = "Product deleted successfully"),
        (status = 404, description = "Product not found", body = AuthErrorResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn delete_product(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager"])?;

    let p = product::Entity::find()
        .filter(product::Column::Id.eq(id))
        .filter(product::Column::TenantId.eq(auth.tenant_id.as_uuid()))
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
                    error: "Product not found".to_string(),
                }),
            )
        })?;

    let active: product::ActiveModel = p.into();
    active.delete(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to delete product: {}", e),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({
        "status": "deleted",
        "product_id": id
    })))
}
