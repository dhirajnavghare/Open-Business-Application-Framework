use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use obaf_core::{generate_token, hash_password, verify_password, TenantId};
use obaf_database::entities::{organization, user};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct LoginResponse {
    pub token: String,
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub email: String,
    pub full_name: String,
    pub role: String,
}

#[derive(Deserialize, ToSchema)]
pub struct RegisterUserRequest {
    pub organization_id: Uuid,
    pub email: String,
    pub password: String,
    pub full_name: String,
    pub role: String, // "admin", "manager", "staff", "cashier"
}

#[derive(Serialize, ToSchema)]
pub struct UserProfileResponse {
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub email: String,
    pub full_name: String,
    pub role: String,
    pub is_active: bool,
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "Auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful, returns JWT bearer token", body = LoginResponse),
        (status = 401, description = "Invalid email or password", body = AuthErrorResponse)
    )
)]
pub async fn login(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let client_ip = if let Some(ip) = headers
        .get("cf-connecting-ip")
        .and_then(|v| v.to_str().ok())
    {
        ip.trim().to_string()
    } else if let Some(ip) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        ip.trim().to_string()
    } else if let Some(ip) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        ip.split(',')
            .next_back()
            .unwrap_or("127.0.0.1")
            .trim()
            .to_string()
    } else {
        "127.0.0.1".to_string()
    };

    let email_cleaned = payload.email.trim().to_lowercase();
    let attempt_key = format!("tenant_login:{}:{}", email_cleaned, client_ip);

    // Defense-in-Depth: Progressive brute force and credential stuffing mitigation
    if let Err(retry_secs) = state.login_guard.check_lockout(&attempt_key) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(AuthErrorResponse {
                error: format!(
                    "Too many failed login attempts: account locked. Please retry after {} seconds.",
                    retry_secs
                ),
            }),
        ));
    }

    let user_opt = user::Entity::find()
        .filter(user::Column::Email.eq(&email_cleaned))
        .filter(user::Column::IsActive.eq(true))
        .one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database query error on login: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: "A database error occurred during login".to_string(),
                }),
            )
        })?;

    // Precomputed dummy Argon2id hash to equalize verification timing if user doesn't exist (anti-enumeration)
    const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$ZHVtbXlzYWx0MTIzNDU2$O+xV3lqU4E8m8GvZ05vL4H9fD3P0gZp6l8K4j1s9Q0I";

    let (is_valid, user) = match user_opt {
        Some(u) => {
            let valid = verify_password(&payload.password, &u.password_hash).unwrap_or(false);
            (valid, Some(u))
        }
        None => {
            let _ = verify_password(&payload.password, DUMMY_HASH);
            (false, None)
        }
    };

    if !is_valid || user.is_none() {
        let (_count, is_locked) = state.login_guard.record_failure(&attempt_key);
        let err_msg = if is_locked {
            "Too many failed login attempts: account locked for 15 minutes.".to_string()
        } else {
            "Invalid email or password".to_string()
        };

        return Err((
            if is_locked {
                StatusCode::TOO_MANY_REQUESTS
            } else {
                StatusCode::UNAUTHORIZED
            },
            Json(AuthErrorResponse { error: err_msg }),
        ));
    }

    state.login_guard.record_success(&attempt_key);
    let user = user.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(AuthErrorResponse {
                error: "Invalid email or password".to_string(),
            }),
        )
    })?;

    let token = generate_token(
        user.id,
        TenantId(user.organization_id),
        &user.role,
        &state.jwt_secret,
        24,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to generate authentication token: {}", e),
            }),
        )
    })?;

    Ok(Json(LoginResponse {
        token,
        user_id: user.id,
        organization_id: user.organization_id,
        email: user.email,
        full_name: user.full_name,
        role: user.role,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    tag = "Auth",
    request_body = RegisterUserRequest,
    responses(
        (status = 201, description = "User registered successfully", body = LoginResponse),
        (status = 409, description = "User with email already exists", body = AuthErrorResponse)
    )
)]
pub async fn register(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(payload): Json<RegisterUserRequest>,
) -> Result<(StatusCode, Json<LoginResponse>), (StatusCode, Json<AuthErrorResponse>)> {
    let role_cleaned = payload.role.trim().to_lowercase();
    if role_cleaned == "superadmin" || role_cleaned.contains("superadmin") {
        return Err((
            StatusCode::FORBIDDEN,
            Json(AuthErrorResponse {
                error: "Forbidden: Superadmin role cannot be registered via tenant API".to_string(),
            }),
        ));
    }

    const ALLOWED_ROLES: &[&str] = &["admin", "manager", "staff", "cashier", "viewer", "waiter"];
    if !ALLOWED_ROLES.contains(&role_cleaned.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: format!(
                    "Invalid role '{}'. Allowed roles: {:?}",
                    payload.role, ALLOWED_ROLES
                ),
            }),
        ));
    }

    // Verify organization exists
    let org_exists = organization::Entity::find_by_id(payload.organization_id)
        .one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error checking organization: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: "A database error occurred".to_string(),
                }),
            )
        })?;

    if org_exists.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(AuthErrorResponse {
                error: "Organization not found".to_string(),
            }),
        ));
    }

    // Privilege Escalation Defense:
    // If organization already has active users, adding new users requires an existing tenant admin (or superadmin).
    let existing_users_count = user::Entity::find()
        .filter(user::Column::OrganizationId.eq(payload.organization_id))
        .count(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error counting users: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: "A database error occurred".to_string(),
                }),
            )
        })?;

    if existing_users_count > 0 {
        let auth_token = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "));

        let is_authorized = match auth_token {
            Some(t) => {
                if state.is_token_revoked(t) {
                    false
                } else if let Ok(claims) = obaf_core::verify_token(t, &state.jwt_secret) {
                    claims.role == "superadmin"
                        || (claims.tenant_id.0 == payload.organization_id
                            && (claims.role == "admin" || claims.role == "manager"))
                } else {
                    false
                }
            }
            None => false,
        };

        if !is_authorized {
            return Err((
                StatusCode::FORBIDDEN,
                Json(AuthErrorResponse {
                    error: "Forbidden: Registering users for an existing organization requires tenant admin authorization".to_string(),
                }),
            ));
        }
    }

    let existing = user::Entity::find()
        .filter(user::Column::Email.eq(&payload.email))
        .one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error querying user email: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: "A database error occurred".to_string(),
                }),
            )
        })?;

    if existing.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(AuthErrorResponse {
                error: format!("User with email '{}' already exists", payload.email),
            }),
        ));
    }

    if payload.password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: "Password must be at least 8 characters long".to_string(),
            }),
        ));
    }

    let password_hash = hash_password(&payload.password).map_err(|e| {
        tracing::error!("Failed to hash password: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: "Failed to process credentials securely".to_string(),
            }),
        )
    })?;

    let user_id = Uuid::new_v4();
    let now = Utc::now();

    let new_user = user::ActiveModel {
        id: Set(user_id),
        organization_id: Set(payload.organization_id),
        email: Set(payload.email.clone()),
        password_hash: Set(password_hash),
        full_name: Set(payload.full_name.clone()),
        role: Set(role_cleaned.clone()),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    };

    new_user.insert(&state.db).await.map_err(|e| {
        tracing::error!("Failed to create user: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: "Failed to create user record".to_string(),
            }),
        )
    })?;

    let token = generate_token(
        user_id,
        TenantId(payload.organization_id),
        &role_cleaned,
        &state.jwt_secret,
        24,
    )
    .map_err(|e| {
        tracing::error!("Failed to generate authentication token: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: "Failed to generate authentication token".to_string(),
            }),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(LoginResponse {
            token,
            user_id,
            organization_id: payload.organization_id,
            email: payload.email,
            full_name: payload.full_name,
            role: payload.role,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "Auth",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Current authenticated user profile", body = UserProfileResponse),
        (status = 401, description = "Unauthorized or invalid token", body = AuthErrorResponse)
    )
)]
pub async fn me(
    State(state): State<SharedState>,
    auth: AuthUser,
) -> Result<Json<UserProfileResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let user = user::Entity::find_by_id(auth.user_id)
        .one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database query error fetching user profile: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: "A database error occurred".to_string(),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: "User not found".to_string(),
                }),
            )
        })?;

    let org = organization::Entity::find_by_id(user.organization_id)
        .one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database query error fetching organization: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: "A database error occurred".to_string(),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: "Organization not found".to_string(),
                }),
            )
        })?;

    Ok(Json(UserProfileResponse {
        user_id: user.id,
        organization_id: user.organization_id,
        organization_name: org.name,
        email: user.email,
        full_name: user.full_name,
        role: user.role,
        is_active: user.is_active,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "Auth",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Logged out successfully; session token revoked"),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn logout(
    State(state): State<SharedState>,
    _auth: AuthUser,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<AuthErrorResponse>)> {
    if let Some(auth_header) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            state.revoke_token(token);
        }
    }

    Ok(Json(serde_json::json!({
        "message": "Successfully logged out. Session token has been revoked."
    })))
}
