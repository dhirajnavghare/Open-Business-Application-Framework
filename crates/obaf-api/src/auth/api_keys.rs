use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{Duration, Utc};
use obaf_database::entities::api_key;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub scopes: Option<Vec<String>>,
    pub expires_in_days: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct CreateApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub key: String, // Plaintext key returned only once upon creation
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Serialize, ToSchema)]
pub struct ApiKeySummary {
    pub id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub is_active: bool,
    pub created_at: chrono::DateTime<Utc>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/api-keys",
    tag = "Auth",
    security(("bearer_auth" = [])),
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "API Key generated (secret key shown once)", body = CreateApiKeyResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn create_api_key(
    auth: AuthUser,
    State(state): State<SharedState>,
    Json(payload): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<CreateApiKeyResponse>), (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin"])?;

    let raw_secret = format!("obaf_live_{}", Uuid::new_v4().simple());
    let prefix = format!("{}...", &raw_secret[..14]);

    let mut hasher = Sha256::new();
    hasher.update(raw_secret.as_bytes());
    let key_hash = hex::encode(hasher.finalize());

    let scopes_vec = payload.scopes.unwrap_or_else(|| vec!["*".to_string()]);
    let scopes_str = serde_json::to_string(&scopes_vec).unwrap_or_else(|_| "[\"*\"]".to_string());

    let now = Utc::now();
    let expires_at = payload
        .expires_in_days
        .map(|days| now + Duration::days(days));

    let key_id = Uuid::new_v4();
    let new_key = api_key::ActiveModel {
        id: Set(key_id),
        tenant_id: Set(auth.tenant_id.0),
        name: Set(payload.name.clone()),
        key_prefix: Set(prefix.clone()),
        key_hash: Set(key_hash),
        scopes: Set(scopes_str),
        is_active: Set(true),
        expires_at: Set(expires_at),
        created_at: Set(now),
    };

    new_key.insert(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(CreateApiKeyResponse {
            id: key_id,
            name: payload.name,
            key: raw_secret,
            key_prefix: prefix,
            scopes: scopes_vec,
            created_at: now,
            expires_at,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/api-keys",
    tag = "Auth",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List all active API keys for tenant", body = Vec<ApiKeySummary>),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn list_api_keys(
    auth: AuthUser,
    State(state): State<SharedState>,
) -> Result<Json<Vec<ApiKeySummary>>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin"])?;

    let keys = api_key::Entity::find()
        .filter(api_key::Column::TenantId.eq(auth.tenant_id.0))
        .order_by_desc(api_key::Column::CreatedAt)
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

    let summaries = keys
        .into_iter()
        .map(|k| {
            let scopes = serde_json::from_str(&k.scopes).unwrap_or_else(|_| vec!["*".to_string()]);
            ApiKeySummary {
                id: k.id,
                name: k.name,
                key_prefix: k.key_prefix,
                scopes,
                is_active: k.is_active,
                created_at: k.created_at,
                expires_at: k.expires_at,
            }
        })
        .collect();

    Ok(Json(summaries))
}

#[utoipa::path(
    delete,
    path = "/api/v1/auth/api-keys/{id}",
    tag = "Auth",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "API Key ID to revoke")
    ),
    responses(
        (status = 200, description = "API Key revoked successfully"),
        (status = 404, description = "API Key not found"),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn revoke_api_key(
    auth: AuthUser,
    Path(id): Path<Uuid>,
    State(state): State<SharedState>,
) -> Result<StatusCode, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin"])?;

    let existing = api_key::Entity::find_by_id(id)
        .filter(api_key::Column::TenantId.eq(auth.tenant_id.0))
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

    if let Some(k) = existing {
        let mut active: api_key::ActiveModel = k.into();
        active.is_active = Set(false);
        active.update(&state.db).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;
        Ok(StatusCode::OK)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(AuthErrorResponse {
                error: "API Key not found".to_string(),
            }),
        ))
    }
}
