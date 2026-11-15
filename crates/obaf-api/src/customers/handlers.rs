use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use obaf_database::entities::customer;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct CreateCustomerRequest {
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub tax_id: Option<String>,
    pub address: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateCustomerRequest {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub tax_id: Option<String>,
    pub address: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Serialize, ToSchema)]
pub struct CustomerResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub tax_id: Option<String>,
    pub address: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl From<customer::Model> for CustomerResponse {
    fn from(m: customer::Model) -> Self {
        let meta = serde_json::from_str(&m.metadata).unwrap_or(serde_json::json!({}));
        Self {
            id: m.id,
            tenant_id: m.tenant_id,
            name: m.name,
            phone: m.phone,
            email: m.email,
            tax_id: m.tax_id,
            address: m.address,
            metadata: meta,
            created_at: m.created_at,
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/customers",
    tag = "Customers",
    security(("bearer_auth" = [])),
    request_body = CreateCustomerRequest,
    responses(
        (status = 201, description = "Customer created successfully", body = CustomerResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn create_customer(
    State(state): State<SharedState>,
    auth: AuthUser,
    Json(payload): Json<CreateCustomerRequest>,
) -> Result<(StatusCode, Json<CustomerResponse>), (StatusCode, Json<AuthErrorResponse>)> {
    let customer_id = Uuid::new_v4();
    let now = Utc::now();
    let meta_str = payload
        .metadata
        .map(|v| v.to_string())
        .unwrap_or_else(|| "{}".to_string());

    let new_customer = customer::ActiveModel {
        id: Set(customer_id),
        tenant_id: Set(auth.tenant_id.as_uuid()),
        name: Set(payload.name),
        phone: Set(payload.phone),
        email: Set(payload.email),
        tax_id: Set(payload.tax_id),
        address: Set(payload.address),
        metadata: Set(meta_str),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let inserted = new_customer.insert(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to create customer: {}", e),
            }),
        )
    })?;

    Ok((StatusCode::CREATED, Json(inserted.into())))
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct CustomerFilterQuery {
    pub search: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/customers",
    tag = "Customers",
    security(("bearer_auth" = [])),
    params(
        CustomerFilterQuery
    ),
    responses(
        (status = 200, description = "List of customers", body = Vec<CustomerResponse>),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn list_customers(
    State(state): State<SharedState>,
    auth: AuthUser,
    Query(query): Query<CustomerFilterQuery>,
) -> Result<Json<Vec<CustomerResponse>>, (StatusCode, Json<AuthErrorResponse>)> {
    let mut q =
        customer::Entity::find().filter(customer::Column::TenantId.eq(auth.tenant_id.as_uuid()));

    if let Some(s) = query.search {
        let pattern = format!("%{}%", s);
        q = q.filter(
            customer::Column::Name
                .like(&pattern)
                .or(customer::Column::Email.like(&pattern))
                .or(customer::Column::Phone.like(&pattern)),
        );
    }

    if let Some(limit) = query.limit {
        q = q.limit(limit);
    }

    if let Some(offset) = query.offset {
        q = q.offset(offset);
    }

    let customers = q.all(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Database error: {}", e),
            }),
        )
    })?;

    let resp = customers.into_iter().map(Into::into).collect();
    Ok(Json(resp))
}

#[utoipa::path(
    get,
    path = "/api/v1/customers/{id}",
    tag = "Customers",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Customer ID")
    ),
    responses(
        (status = 200, description = "Customer details", body = CustomerResponse),
        (status = 404, description = "Customer not found", body = AuthErrorResponse)
    )
)]
pub async fn get_customer(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<CustomerResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let c = customer::Entity::find()
        .filter(customer::Column::Id.eq(id))
        .filter(customer::Column::TenantId.eq(auth.tenant_id.as_uuid()))
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
                    error: "Customer not found".to_string(),
                }),
            )
        })?;

    Ok(Json(c.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/customers/{id}",
    tag = "Customers",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Customer ID")
    ),
    request_body = UpdateCustomerRequest,
    responses(
        (status = 200, description = "Customer updated successfully", body = CustomerResponse),
        (status = 404, description = "Customer not found", body = AuthErrorResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn update_customer(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateCustomerRequest>,
) -> Result<Json<CustomerResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let c = customer::Entity::find()
        .filter(customer::Column::Id.eq(id))
        .filter(customer::Column::TenantId.eq(auth.tenant_id.as_uuid()))
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
                    error: "Customer not found".to_string(),
                }),
            )
        })?;

    let mut active: customer::ActiveModel = c.into();

    if let Some(name) = payload.name {
        active.name = Set(name);
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
    active.updated_at = Set(Utc::now());

    let updated = active.update(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to update customer: {}", e),
            }),
        )
    })?;

    Ok(Json(updated.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/customers/{id}",
    tag = "Customers",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Customer ID")
    ),
    responses(
        (status = 200, description = "Customer deleted successfully"),
        (status = 404, description = "Customer not found", body = AuthErrorResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn delete_customer(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager"])?;

    let c = customer::Entity::find()
        .filter(customer::Column::Id.eq(id))
        .filter(customer::Column::TenantId.eq(auth.tenant_id.as_uuid()))
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
                    error: "Customer not found".to_string(),
                }),
            )
        })?;

    let active: customer::ActiveModel = c.into();
    active.delete(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to delete customer: {}", e),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({
        "status": "deleted",
        "customer_id": id
    })))
}
