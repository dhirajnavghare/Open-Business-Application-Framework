use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use obaf_core::{generate_token, hash_password, verify_password, TenantId};
use obaf_database::entities::{organization, user};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
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
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let user = user::Entity::find()
        .filter(user::Column::Email.eq(&payload.email))
        .filter(user::Column::IsActive.eq(true))
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
                StatusCode::UNAUTHORIZED,
                Json(AuthErrorResponse {
                    error: "Invalid email or password".to_string(),
                }),
            )
        })?;

    let is_valid = verify_password(&payload.password, &user.password_hash).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(AuthErrorResponse {
                error: "Invalid email or password".to_string(),
            }),
        )
    })?;

    if !is_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(AuthErrorResponse {
                error: "Invalid email or password".to_string(),
            }),
        ));
    }

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
    Json(payload): Json<RegisterUserRequest>,
) -> Result<(StatusCode, Json<LoginResponse>), (StatusCode, Json<AuthErrorResponse>)> {
    let existing = user::Entity::find()
        .filter(user::Column::Email.eq(&payload.email))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Database error: {}", e),
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
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to hash password: {}", e),
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
        role: Set(payload.role.clone()),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    };

    new_user.insert(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to create user: {}", e),
            }),
        )
    })?;

    let token = generate_token(
        user_id,
        TenantId(payload.organization_id),
        &payload.role,
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
                    error: "User not found".to_string(),
                }),
            )
        })?;

    let org = organization::Entity::find_by_id(user.organization_id)
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
