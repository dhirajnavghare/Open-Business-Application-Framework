use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use obaf_database::entities::supplier;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct CreateSupplierRequest {
    pub name: String,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub tax_id: Option<String>,
    pub address: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateSupplierRequest {
    pub name: Option<String>,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub tax_id: Option<String>,
    pub address: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub is_active: Option<bool>,
}

#[derive(Serialize, ToSchema)]
pub struct SupplierResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub tax_id: Option<String>,
    pub address: Option<String>,
    pub metadata: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<supplier::Model> for SupplierResponse {
    fn from(m: supplier::Model) -> Self {
        let meta = serde_json::from_str(&m.metadata).unwrap_or(serde_json::json!({}));
        Self {
            id: m.id,
            tenant_id: m.tenant_id,
            name: m.name,
            contact_person: m.contact_person,
            phone: m.phone,
            email: m.email,
            tax_id: m.tax_id,
            address: m.address,
            metadata: meta,
            is_active: m.is_active,
            created_at: m.created_at,
        }
    }
}

#[derive(Deserialize, IntoParams)]
pub struct SupplierFilterQuery {
    pub search: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[utoipa::path(
    post,
    path = "/api/v1/suppliers",
    tag = "Suppliers",
    security(("bearer_auth" = [])),
    request_body = CreateSupplierRequest,
    responses(
        (status = 201, description = "Supplier successfully onboarded", body = SupplierResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn create_supplier(
    auth: AuthUser,
    State(state): State<SharedState>,
    Json(payload): Json<CreateSupplierRequest>,
) -> Result<(StatusCode, Json<SupplierResponse>), (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager"])?;

    let supplier_id = Uuid::new_v4();
    let now = Utc::now();
    let meta_str = payload
        .metadata
        .map(|v| v.to_string())
        .unwrap_or_else(|| "{}".to_string());

    let new_supplier = supplier::ActiveModel {
        id: Set(supplier_id),
        tenant_id: Set(auth.tenant_id.0),
        name: Set(payload.name),
        contact_person: Set(payload.contact_person),
        phone: Set(payload.phone),
        email: Set(payload.email),
        tax_id: Set(payload.tax_id),
        address: Set(payload.address),
        metadata: Set(meta_str),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let saved = new_supplier.insert(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok((StatusCode::CREATED, Json(saved.into())))
}

#[utoipa::path(
    get,
    path = "/api/v1/suppliers",
    tag = "Suppliers",
    security(("bearer_auth" = [])),
    params(SupplierFilterQuery),
    responses(
        (status = 200, description = "List of suppliers", body = Vec<SupplierResponse>),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn list_suppliers(
    auth: AuthUser,
    State(state): State<SharedState>,
    Query(query): Query<SupplierFilterQuery>,
) -> Result<Json<Vec<SupplierResponse>>, (StatusCode, Json<AuthErrorResponse>)> {
    let mut db_query = supplier::Entity::find()
        .filter(supplier::Column::TenantId.eq(auth.tenant_id.0))
        .filter(supplier::Column::IsActive.eq(true))
        .order_by_desc(supplier::Column::CreatedAt);

    if let Some(ref term) = query.search {
        let pattern = format!("%{}%", term);
        db_query = db_query.filter(supplier::Column::Name.like(&pattern));
    }

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let suppliers = db_query
        .limit(page_size)
        .offset((page - 1) * page_size)
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

    Ok(Json(suppliers.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    get,
    path = "/api/v1/suppliers/{id}",
    tag = "Suppliers",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Supplier ID")
    ),
    responses(
        (status = 200, description = "Supplier retrieved", body = SupplierResponse),
        (status = 404, description = "Supplier not found", body = AuthErrorResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn get_supplier(
    auth: AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<SharedState>,
) -> Result<Json<SupplierResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let supplier = supplier::Entity::find_by_id(id)
        .filter(supplier::Column::TenantId.eq(auth.tenant_id.0))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: format!("Supplier with ID {} not found", id),
                }),
            )
        })?;

    Ok(Json(supplier.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/suppliers/{id}",
    tag = "Suppliers",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Supplier ID")
    ),
    request_body = UpdateSupplierRequest,
    responses(
        (status = 200, description = "Supplier updated successfully", body = SupplierResponse),
        (status = 404, description = "Supplier not found", body = AuthErrorResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn update_supplier(
    auth: AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<SharedState>,
    Json(payload): Json<UpdateSupplierRequest>,
) -> Result<Json<SupplierResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager"])?;

    let supplier = supplier::Entity::find_by_id(id)
        .filter(supplier::Column::TenantId.eq(auth.tenant_id.0))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: format!("Supplier with ID {} not found", id),
                }),
            )
        })?;

    let mut active: supplier::ActiveModel = supplier.into();

    if let Some(name) = payload.name {
        active.name = Set(name);
    }
    if let Some(contact) = payload.contact_person {
        active.contact_person = Set(Some(contact));
    }
    if let Some(phone) = payload.phone {
        active.phone = Set(Some(phone));
    }
    if let Some(email) = payload.email {
        active.email = Set(Some(email));
    }
    if let Some(tax_id) = payload.tax_id {
        active.tax_id = Set(Some(tax_id));
    }
    if let Some(address) = payload.address {
        active.address = Set(Some(address));
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
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(updated.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/suppliers/{id}",
    tag = "Suppliers",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Supplier ID")
    ),
    responses(
        (status = 200, description = "Supplier deleted successfully"),
        (status = 404, description = "Supplier not found", body = AuthErrorResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn delete_supplier(
    auth: AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<SharedState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager"])?;

    let supplier = supplier::Entity::find_by_id(id)
        .filter(supplier::Column::TenantId.eq(auth.tenant_id.0))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: format!("Supplier with ID {} not found", id),
                }),
            )
        })?;

    let active: supplier::ActiveModel = supplier.into();
    active.delete(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({
        "status": "deleted",
        "supplier_id": id
    })))
}
