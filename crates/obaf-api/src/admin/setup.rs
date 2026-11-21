use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use obaf_database::{entities::organization, entities::user};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, ModelTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct BootstrapPresetRequest {
    pub organization_name: String,
    pub slug: String,
    pub preset: String, // "restaurant_pos", "pharmacy", "gym_fitness", etc.
    pub admin_email: String,
    pub admin_name: String,
    pub admin_password: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct BootstrapPresetResponse {
    pub organization_id: Uuid,
    pub organization_name: String,
    pub slug: String,
    pub preset: String,
    pub active_modules: Vec<String>,
    pub admin_user_id: Uuid,
    pub message: String,
}

#[derive(Serialize, ToSchema)]
pub struct ApiErrorResponse {
    pub error: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/admin/setup/preset",
    tag = "Admin",
    request_body = BootstrapPresetRequest,
    responses(
        (status = 201, description = "Organization successfully initialized with preset", body = BootstrapPresetResponse),
        (status = 400, description = "Invalid request or preset not found", body = ApiErrorResponse),
        (status = 409, description = "Organization slug or email already exists", body = ApiErrorResponse)
    )
)]
pub async fn bootstrap_preset(
    State(state): State<SharedState>,
    Json(payload): Json<BootstrapPresetRequest>,
) -> Result<(StatusCode, Json<BootstrapPresetResponse>), (StatusCode, Json<ApiErrorResponse>)> {
    // 1. Validate slug and preset inputs to prevent injection and path traversal attacks
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

    let preset = payload.preset.trim().to_lowercase();
    if preset.is_empty()
        || preset.len() > 64
        || !preset
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
    let raw_password = payload
        .admin_password
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiErrorResponse {
                    error: "admin_password is required to bootstrap an organization".to_string(),
                }),
            )
        })?;

    if raw_password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiErrorResponse {
                error: "Admin password must be at least 8 characters long".to_string(),
            }),
        ));
    }

    // 2. Verify slug uniqueness
    let existing = organization::Entity::find()
        .filter(organization::Column::Slug.eq(&slug))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?;

    if existing.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(ApiErrorResponse {
                error: format!("Organization with slug '{}' already exists", slug),
            }),
        ));
    }

    // Check admin email uniqueness across platform users
    let existing_email = user::Entity::find()
        .filter(user::Column::Email.eq(&email_cleaned))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?;

    if existing_email.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(ApiErrorResponse {
                error: format!("User with email '{}' already exists", email_cleaned),
            }),
        ));
    }

    let password_hash = obaf_core::hash_password(raw_password).map_err(|e| {
        tracing::error!("Failed to hash admin password: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiErrorResponse {
                error: "Failed to process credentials securely".to_string(),
            }),
        )
    })?;

    // 3. Load preset configuration from config/presets/*.json if available
    let preset_file_path = std::path::Path::new("config/presets").join(format!("{}.json", preset));
    let (active_modules, settings_json) =
        if let Ok(content) = std::fs::read_to_string(&preset_file_path) {
            if let Ok(preset_json) = serde_json::from_str::<serde_json::Value>(&content) {
                let modules = preset_json
                    .get("active_modules")
                    .and_then(|m| m.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect::<Vec<String>>()
                    })
                    .unwrap_or_else(|| {
                        vec![
                            "catalog".to_string(),
                            "orders".to_string(),
                            "billing".to_string(),
                        ]
                    });
                (modules, content)
            } else {
                (
                    vec![
                        "catalog".to_string(),
                        "orders".to_string(),
                        "billing".to_string(),
                    ],
                    "{}".to_string(),
                )
            }
        } else {
            let default_modules = match payload.preset.as_str() {
                "restaurant_pos" => vec![
                    "catalog".to_string(),
                    "orders".to_string(),
                    "kot".to_string(),
                    "billing".to_string(),
                    "tables".to_string(),
                ],
                "pharmacy" => vec![
                    "catalog".to_string(),
                    "inventory".to_string(),
                    "batches".to_string(),
                    "billing".to_string(),
                    "prescriptions".to_string(),
                ],
                _ => vec![
                    "catalog".to_string(),
                    "orders".to_string(),
                    "billing".to_string(),
                ],
            };
            (default_modules, "{}".to_string())
        };

    let org_id = Uuid::new_v4();
    let now = Utc::now();

    // 3. Insert organization
    let org = organization::ActiveModel {
        id: Set(org_id),
        name: Set(payload.organization_name.clone()),
        slug: Set(slug.clone()),
        preset: Set(preset.clone()),
        active_modules: Set(serde_json::to_string(&active_modules).unwrap_or_default()),
        settings: Set(settings_json),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let inserted_org = org.insert(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiErrorResponse {
                error: format!("Failed to create organization: {}", e),
            }),
        )
    })?;

    // 4. Create initial Admin user
    let user_id = Uuid::new_v4();
    let user = user::ActiveModel {
        id: Set(user_id),
        organization_id: Set(org_id),
        email: Set(email_cleaned),
        password_hash: Set(password_hash),
        full_name: Set(payload.admin_name),
        role: Set("admin".to_string()),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    };

    if let Err(e) = user.insert(&state.db).await {
        let _ = inserted_org.delete(&state.db).await;
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiErrorResponse {
                error: format!("Failed to create admin user: {}", e),
            }),
        ));
    }

    super::audit::record_audit_log(
        &state.db,
        super::audit::NewAuditEntry {
            tenant_id: org_id,
            actor_id: Some(user_id),
            action: "PRESET_BOOTSTRAPPED".to_string(),
            entity_type: "organization".to_string(),
            entity_id: Some(org_id.to_string()),
            details: serde_json::json!({
                "organization_name": payload.organization_name,
                "preset": payload.preset,
                "admin_user_id": user_id
            }),
            ip_address: None,
        },
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(BootstrapPresetResponse {
            organization_id: org_id,
            organization_name: payload.organization_name,
            slug,
            preset,
            active_modules,
            admin_user_id: user_id,
            message: "Organization and admin user successfully bootstrapped".to_string(),
        }),
    ))
}
