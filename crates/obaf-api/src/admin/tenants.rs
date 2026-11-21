use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use obaf_core::security::{generate_token, hash_password};
use obaf_database::entities::{order, organization, product, user};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, ModelTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::admin::setup::ApiErrorResponse;
use crate::auth::extractor::AuthUser;
use crate::state::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct ProvisionTenantRequest {
    pub organization_name: String,
    pub slug: String,
    pub preset_id: Option<String>,
    pub plan_tier: Option<String>, // "starter", "growth", "enterprise"
    pub admin_email: String,
    pub admin_name: String,
    pub admin_password: Option<String>,
    pub active_modules: Option<Vec<String>>,
}

#[derive(Serialize, ToSchema)]
pub struct ProvisionTenantResponse {
    pub tenant_id: Uuid,
    pub organization_name: String,
    pub slug: String,
    pub plan_tier: String,
    pub admin_user_id: Uuid,
    pub admin_email: String,
    pub preset_applied: Option<String>,
    pub access_token: String,
}

#[derive(Serialize, ToSchema)]
pub struct TenantSummary {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub preset: String,
    pub plan_tier: String,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct TenantUsageResponse {
    pub tenant_id: Uuid,
    pub organization_name: String,
    pub plan_tier: String,
    pub products_count: usize,
    pub products_limit: Option<usize>,
    pub orders_count: usize,
    pub orders_limit: Option<usize>,
    pub users_count: usize,
    pub users_limit: Option<usize>,
    pub is_within_quota: bool,
}

#[utoipa::path(
    post,
    path = "/api/v1/admin/tenants",
    tag = "Admin",
    request_body = ProvisionTenantRequest,
    responses(
        (status = 201, description = "SaaS tenant provisioned successfully", body = ProvisionTenantResponse),
        (status = 400, description = "Invalid request or preset not found", body = ApiErrorResponse),
        (status = 409, description = "Organization slug or admin email already exists", body = ApiErrorResponse)
    )
)]
pub async fn provision_tenant(
    State(state): State<SharedState>,
    Json(payload): Json<ProvisionTenantRequest>,
) -> Result<(StatusCode, Json<ProvisionTenantResponse>), (StatusCode, Json<ApiErrorResponse>)> {
    // Validate slug format
    let slug = payload.slug.trim().to_lowercase();
    if slug.is_empty()
        || slug.len() > 64
        || !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiErrorResponse {
                error: "Invalid slug format: must contain only lowercase alphanumeric characters and hyphens (1-64 chars)".to_string(),
            }),
        ));
    }

    // Check slug uniqueness
    let existing_org = organization::Entity::find()
        .filter(organization::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    if existing_org.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(ApiErrorResponse {
                error: format!("Organization with slug '{}' already exists", slug),
            }),
        ));
    }

    let preset_id = payload
        .preset_id
        .clone()
        .unwrap_or_else(|| "retail_grocery".to_string())
        .trim()
        .to_lowercase();

    if !preset_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiErrorResponse {
                error: "Invalid preset format: must contain only lowercase alphanumeric characters and underscores".to_string(),
            }),
        ));
    }

    let email_cleaned = payload.admin_email.trim().to_lowercase();
    if email_cleaned.is_empty() || !email_cleaned.contains('@') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiErrorResponse {
                error: "A valid admin email address is required".to_string(),
            }),
        ));
    }

    // Defense-in-depth: Require an explicit, strong tenant admin password (CWE-1188 defense)
    let raw_pwd = payload
        .admin_password
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiErrorResponse {
                    error: "admin_password is required when provisioning a tenant".to_string(),
                }),
            )
        })?;

    if raw_pwd.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiErrorResponse {
                error: "Tenant admin password must be at least 8 characters long".to_string(),
            }),
        ));
    }

    // Check tenant admin email uniqueness across platform users
    let existing_user = user::Entity::find()
        .filter(user::Column::Email.eq(&email_cleaned))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    if existing_user.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(ApiErrorResponse {
                error: format!("User with email '{}' already exists", email_cleaned),
            }),
        ));
    }

    let hashed_pwd = hash_password(raw_pwd).map_err(|e| {
        tracing::error!("Password hashing failed: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiErrorResponse {
                error: "Failed to process credentials securely".to_string(),
            }),
        )
    })?;

    let plan_tier = payload
        .plan_tier
        .clone()
        .unwrap_or_else(|| "starter".to_string());

    let org_id = Uuid::new_v4();
    let now = Utc::now();
    let settings = serde_json::json!({
        "plan_tier": plan_tier,
        "provisioned_via": "saas_admin"
    });

    let default_all_modules: Vec<String> = crate::superadmin::organizations::ALL_MODULES
        .iter()
        .map(|m| m.id.to_string())
        .collect();
    let modules = payload.active_modules.unwrap_or(default_all_modules);
    let modules_json = serde_json::to_string(&modules).unwrap_or_else(|_| "[]".to_string());

    let new_org = organization::ActiveModel {
        id: Set(org_id),
        slug: Set(slug.clone()),
        name: Set(payload.organization_name.clone()),
        preset: Set(preset_id.clone()),
        active_modules: Set(modules_json.clone()),
        settings: Set(settings.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let inserted_org = new_org.insert(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiErrorResponse {
                error: format!("Failed to create organization: {}", e),
            }),
        )
    })?;

    state
        .cache
        .insert(format!("tenant_modules:{}", org_id), modules_json);

    // Create admin user
    let user_id = Uuid::new_v4();
    let new_user = user::ActiveModel {
        id: Set(user_id),
        organization_id: Set(org_id),
        email: Set(email_cleaned),
        password_hash: Set(hashed_pwd),
        full_name: Set(payload.admin_name.clone()),
        role: Set("admin".to_string()),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    };

    if let Err(e) = new_user.insert(&state.db).await {
        let _ = inserted_org.delete(&state.db).await;
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiErrorResponse {
                error: format!("Failed to create admin user: {}", e),
            }),
        ));
    }

    let token = generate_token(
        user_id,
        obaf_core::TenantId(org_id),
        "admin",
        &state.jwt_secret,
        24,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiErrorResponse {
                error: format!("Failed to generate auth token: {}", e),
            }),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(ProvisionTenantResponse {
            tenant_id: org_id,
            organization_name: payload.organization_name,
            slug,
            plan_tier,
            admin_user_id: user_id,
            admin_email: payload.admin_email,
            preset_applied: Some(preset_id),
            access_token: token,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/tenants",
    tag = "Admin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of all SaaS tenants", body = Vec<TenantSummary>),
        (status = 401, description = "Unauthorized", body = ApiErrorResponse),
        (status = 403, description = "Forbidden: admin role required", body = ApiErrorResponse)
    )
)]
pub async fn list_tenants(
    State(state): State<SharedState>,
    auth: AuthUser,
) -> Result<Json<Vec<TenantSummary>>, (StatusCode, Json<ApiErrorResponse>)> {
    auth.require_role(&["admin"])
        .map_err(|(sc, res)| (sc, Json(ApiErrorResponse { error: res.0.error })))?;

    let mut q = organization::Entity::find();
    if !auth.is_superadmin() {
        q = q.filter(organization::Column::Id.eq(auth.tenant_id.as_uuid()));
    }

    let orgs = q
        .order_by_desc(organization::Column::CreatedAt)
        .all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error fetching tenants: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse {
                    error: "A database error occurred".to_string(),
                }),
            )
        })?;

    let summaries = orgs
        .into_iter()
        .map(|o| {
            let plan_tier = serde_json::from_str::<serde_json::Value>(&o.settings)
                .ok()
                .and_then(|v| {
                    v.get("plan_tier")
                        .and_then(|p| p.as_str().map(|s| s.to_string()))
                })
                .unwrap_or_else(|| "starter".to_string());

            TenantSummary {
                id: o.id,
                name: o.name,
                slug: o.slug,
                preset: o.preset,
                plan_tier,
                created_at: o.created_at,
            }
        })
        .collect();

    Ok(Json(summaries))
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/tenants/{id}/usage",
    tag = "Admin",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Tenant / Organization ID")
    ),
    responses(
        (status = 200, description = "Usage metrics and plan entitlements quota status", body = TenantUsageResponse),
        (status = 401, description = "Unauthorized", body = ApiErrorResponse),
        (status = 403, description = "Forbidden: cross-tenant access denied", body = ApiErrorResponse),
        (status = 404, description = "Tenant not found", body = ApiErrorResponse)
    )
)]
pub async fn get_tenant_usage(
    Path(id): Path<Uuid>,
    State(state): State<SharedState>,
    auth: AuthUser,
) -> Result<Json<TenantUsageResponse>, (StatusCode, Json<ApiErrorResponse>)> {
    // Cross-tenant BOLA/IDOR protection
    if !auth.is_superadmin() && auth.tenant_id.as_uuid() != id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiErrorResponse {
                error: "Forbidden: Cannot access usage metrics of another organization".to_string(),
            }),
        ));
    }

    let org = organization::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error fetching tenant usage: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse {
                    error: "A database error occurred".to_string(),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ApiErrorResponse {
                    error: format!("Tenant with ID {} not found", id),
                }),
            )
        })?;

    let plan_tier = serde_json::from_str::<serde_json::Value>(&org.settings)
        .ok()
        .and_then(|v| {
            v.get("plan_tier")
                .and_then(|p| p.as_str().map(|s| s.to_string()))
        })
        .unwrap_or_else(|| "starter".to_string());

    let (products_limit, orders_limit, users_limit) = match plan_tier.to_lowercase().as_str() {
        "starter" => (Some(100), Some(500), Some(3)),
        "growth" => (Some(5000), Some(20000), Some(25)),
        _ => (None, None, None), // Enterprise: unlimited
    };

    let products_count = product::Entity::find()
        .filter(product::Column::TenantId.eq(id))
        .count(&state.db)
        .await
        .unwrap_or(0) as usize;

    let orders_count = order::Entity::find()
        .filter(order::Column::TenantId.eq(id))
        .count(&state.db)
        .await
        .unwrap_or(0) as usize;

    let users_count = user::Entity::find()
        .filter(user::Column::OrganizationId.eq(id))
        .count(&state.db)
        .await
        .unwrap_or(0) as usize;

    let is_within_quota = products_limit.is_none_or(|l| products_count <= l)
        && orders_limit.is_none_or(|l| orders_count <= l)
        && users_limit.is_none_or(|l| users_count <= l);

    Ok(Json(TenantUsageResponse {
        tenant_id: id,
        organization_name: org.name,
        plan_tier,
        products_count,
        products_limit,
        orders_count,
        orders_limit,
        users_count,
        users_limit,
        is_within_quota,
    }))
}
