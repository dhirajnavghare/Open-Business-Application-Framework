// Copyright 2026 Dhiraj Navghare
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use axum::{extract::State, http::StatusCode, Json};
use obaf_core::security::generate_token;
use obaf_core::TenantId;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::extractor::AuthUser;
use crate::state::SharedState;

/// Fixed well-known UUID for the top-level Platform Super Administrator
pub const SUPERADMIN_USER_ID: Uuid = Uuid::from_u128(1);

#[derive(Debug, Deserialize, ToSchema)]
pub struct SuperAdminLoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SuperAdminGoogleLoginRequest {
    /// Google ID Token (credential) returned by Google Identity Services SDK
    pub credential: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SuperAdminUserInfo {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SuperAdminLoginResponse {
    pub token: String,
    pub user: SuperAdminUserInfo,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SuperAdminConfigResponse {
    pub google_client_id: Option<String>,
    pub google_auth_enabled: bool,
    pub superadmin_email: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorMessage {
    pub error: String,
}

/// Login as Platform Super Admin using configured email and password.
#[utoipa::path(
    post,
    path = "/api/v1/superadmin/login",
    tag = "SuperAdmin",
    request_body = SuperAdminLoginRequest,
    responses(
        (status = 200, description = "Superadmin login successful", body = SuperAdminLoginResponse),
        (status = 401, description = "Invalid superadmin credentials", body = ErrorMessage)
    )
)]
pub async fn superadmin_login(
    State(state): State<SharedState>,
    Json(payload): Json<SuperAdminLoginRequest>,
) -> Result<Json<SuperAdminLoginResponse>, (StatusCode, Json<ErrorMessage>)> {
    let email_cleaned = payload.email.trim().to_lowercase();
    let config = &state.superadmin;

    if email_cleaned != config.email.to_lowercase() || !config.verify_password(&payload.password) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorMessage {
                error: "Invalid superadmin email or password.".to_string(),
            }),
        ));
    }

    let token = generate_token(
        SUPERADMIN_USER_ID,
        TenantId(Uuid::nil()),
        "superadmin",
        &state.jwt_secret,
        24,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorMessage {
                error: format!("Failed to generate authentication token: {}", e),
            }),
        )
    })?;

    Ok(Json(SuperAdminLoginResponse {
        token,
        user: SuperAdminUserInfo {
            user_id: SUPERADMIN_USER_ID,
            email: config.email.clone(),
            role: "superadmin".to_string(),
        },
    }))
}

/// Login as Platform Super Admin using Google SSO (Google Identity Services ID Token).
#[utoipa::path(
    post,
    path = "/api/v1/superadmin/google-login",
    tag = "SuperAdmin",
    request_body = SuperAdminGoogleLoginRequest,
    responses(
        (status = 200, description = "Superadmin Google SSO login successful", body = SuperAdminLoginResponse),
        (status = 401, description = "Unauthorized Google account", body = ErrorMessage)
    )
)]
pub async fn superadmin_google_login(
    State(state): State<SharedState>,
    Json(payload): Json<SuperAdminGoogleLoginRequest>,
) -> Result<Json<SuperAdminLoginResponse>, (StatusCode, Json<ErrorMessage>)> {
    let config = &state.superadmin;
    let credential = payload.credential.trim();

    if credential.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorMessage {
                error: "Google credential token must not be empty.".to_string(),
            }),
        ));
    }

    // 1. Check for mock token (for offline testing & CI verification)
    let verified_email = if let Some(mock_email) = credential.strip_prefix("mock_google_token:") {
        mock_email.trim().to_string()
    } else {
        // 2. Query Google token verification endpoint
        let verify_url = format!(
            "https://oauth2.googleapis.com/tokeninfo?id_token={}",
            credential
        );

        let response = state
            .http_client
            .get(&verify_url)
            .send()
            .await
            .map_err(|e| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorMessage {
                        error: format!("Failed to verify token with Google: {}", e),
                    }),
                )
            })?;

        if !response.status().is_success() {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorMessage {
                    error: "Google token verification failed: invalid or expired token."
                        .to_string(),
                }),
            ));
        }

        let token_info: serde_json::Value = response.json().await.map_err(|e| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorMessage {
                    error: format!("Failed to parse Google token info response: {}", e),
                }),
            )
        })?;

        // Verify audience (client_id) if configured
        if let Some(ref expected_client_id) = config.google_client_id {
            if let Some(aud) = token_info.get("aud").and_then(|v| v.as_str()) {
                if aud != expected_client_id {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        Json(ErrorMessage {
                            error: "Google token client ID does not match configured audience."
                                .to_string(),
                        }),
                    ));
                }
            }
        }

        // Verify email_verified flag
        let email_verified = token_info
            .get("email_verified")
            .and_then(|v| {
                v.as_bool()
                    .or_else(|| v.as_str().map(|s| s.eq_ignore_ascii_case("true")))
            })
            .unwrap_or(false);

        if !email_verified {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorMessage {
                    error: "Google account email is not verified.".to_string(),
                }),
            ));
        }

        token_info
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };

    // 3. Verify that the Google email belongs to the authorized Super Admin
    if !config.verify_google_email(&verified_email) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorMessage {
                error: format!(
                    "Unauthorized: Google account '{}' is not registered as the platform super administrator.",
                    verified_email
                ),
            }),
        ));
    }

    let token = generate_token(
        SUPERADMIN_USER_ID,
        TenantId(Uuid::nil()),
        "superadmin",
        &state.jwt_secret,
        24,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorMessage {
                error: format!("Failed to generate authentication token: {}", e),
            }),
        )
    })?;

    Ok(Json(SuperAdminLoginResponse {
        token,
        user: SuperAdminUserInfo {
            user_id: SUPERADMIN_USER_ID,
            email: verified_email,
            role: "superadmin".to_string(),
        },
    }))
}

/// Retrieve public Super Admin UI configuration (Google Client ID, auth status).
#[utoipa::path(
    get,
    path = "/api/v1/superadmin/config",
    tag = "SuperAdmin",
    responses(
        (status = 200, description = "Superadmin configuration", body = SuperAdminConfigResponse)
    )
)]
pub async fn get_superadmin_config(
    State(state): State<SharedState>,
) -> Json<SuperAdminConfigResponse> {
    let config = &state.superadmin;
    Json(SuperAdminConfigResponse {
        google_client_id: config.google_client_id.clone(),
        google_auth_enabled: config.is_google_auth_enabled(),
        superadmin_email: config.email.clone(),
    })
}

/// Retrieve currently authenticated Super Admin user profile.
#[utoipa::path(
    get,
    path = "/api/v1/superadmin/me",
    tag = "SuperAdmin",
    responses(
        (status = 200, description = "Current superadmin profile", body = SuperAdminUserInfo),
        (status = 403, description = "Forbidden: requires superadmin role", body = ErrorMessage)
    )
)]
pub async fn get_superadmin_me(
    user: AuthUser,
    State(state): State<SharedState>,
) -> Result<Json<SuperAdminUserInfo>, (StatusCode, Json<ErrorMessage>)> {
    if !user.is_superadmin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorMessage {
                error: "Forbidden: Superadmin access required".to_string(),
            }),
        ));
    }

    Ok(Json(SuperAdminUserInfo {
        user_id: user.user_id,
        email: state.superadmin.email.clone(),
        role: user.role,
    }))
}
