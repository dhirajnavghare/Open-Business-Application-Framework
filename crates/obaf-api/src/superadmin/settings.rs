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
use obaf_database::{get_system_setting, set_system_setting};
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::str::FromStr;
use utoipa::ToSchema;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Debug, Serialize, ToSchema)]
pub struct PlatformSettingsResponse {
    pub platform_name: String,
    pub default_currency: String,
    pub token_expiry_hours: u64,
    pub google_client_id: Option<String>,
    pub google_sso_active: bool,
    pub database_type: String,
    pub server_version: String,
    pub superadmin_email: String,
    pub allowed_ips: String,
    pub mfa_enabled: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePlatformSettingsRequest {
    pub platform_name: Option<String>,
    pub default_currency: Option<String>,
    pub token_expiry_hours: Option<u64>,
    pub google_client_id: Option<String>,
    pub allowed_ips: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/superadmin/settings",
    tag = "SuperAdmin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current platform settings", body = PlatformSettingsResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse),
        (status = 403, description = "Forbidden: superadmin required", body = AuthErrorResponse)
    )
)]
pub async fn get_platform_settings(
    auth: AuthUser,
    State(state): State<SharedState>,
) -> Result<Json<PlatformSettingsResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_superadmin()?;

    let db_backend = match state.db.get_database_backend() {
        sea_orm::DbBackend::Sqlite => "SQLite (Zero-Dependency Embedded)",
        sea_orm::DbBackend::Postgres => "PostgreSQL (Enterprise Clustered)",
        sea_orm::DbBackend::MySql => "MySQL",
    };

    let platform_name = get_system_setting(&state.db, "platform_name")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "Open Business Application Framework".to_string());

    let default_currency = get_system_setting(&state.db, "default_currency")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "USD".to_string());

    let token_expiry_hours = get_system_setting(&state.db, "token_expiry_hours")
        .await
        .ok()
        .flatten()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(24);

    let google_client_id = match state.superadmin.google_client_id.clone() {
        Some(cid) => Some(cid),
        None => get_system_setting(&state.db, "google_client_id")
            .await
            .ok()
            .flatten(),
    };

    let google_sso_active = google_client_id.is_some();

    let allowed_ips = get_system_setting(&state.db, "superadmin_allowed_ips")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    let mfa_enabled = get_system_setting(&state.db, "superadmin_totp_secret")
        .await
        .ok()
        .flatten()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    Ok(Json(PlatformSettingsResponse {
        platform_name,
        default_currency,
        token_expiry_hours,
        google_client_id,
        google_sso_active,
        database_type: db_backend.to_string(),
        server_version: state.version.to_string(),
        superadmin_email: state.superadmin.email.clone(),
        allowed_ips,
        mfa_enabled,
    }))
}

#[utoipa::path(
    put,
    path = "/api/v1/superadmin/settings",
    tag = "SuperAdmin",
    security(("bearer_auth" = [])),
    request_body = UpdatePlatformSettingsRequest,
    responses(
        (status = 200, description = "Updated platform settings", body = PlatformSettingsResponse),
        (status = 400, description = "Invalid settings data", body = AuthErrorResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse),
        (status = 403, description = "Forbidden: superadmin required", body = AuthErrorResponse)
    )
)]
pub async fn update_platform_settings(
    auth: AuthUser,
    State(state): State<SharedState>,
    Json(payload): Json<UpdatePlatformSettingsRequest>,
) -> Result<Json<PlatformSettingsResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_superadmin()?;

    if let Some(name) = &payload.platform_name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(AuthErrorResponse {
                    error: "Platform name cannot be empty".to_string(),
                }),
            ));
        }
        set_system_setting(
            &state.db,
            "platform_name",
            trimmed,
            Some("Platform display name branding"),
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Failed to save platform name: {}", e),
                }),
            )
        })?;
    }

    if let Some(curr) = &payload.default_currency {
        let trimmed = curr.trim().to_uppercase();
        if trimmed.len() != 3 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(AuthErrorResponse {
                    error: "Currency must be a valid 3-letter ISO code (e.g. USD, EUR, INR)"
                        .to_string(),
                }),
            ));
        }
        set_system_setting(
            &state.db,
            "default_currency",
            &trimmed,
            Some("Default base billing currency"),
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Failed to save default currency: {}", e),
                }),
            )
        })?;
    }

    if let Some(hours) = payload.token_expiry_hours {
        if hours == 0 || hours > 8760 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(AuthErrorResponse {
                    error: "Token expiry must be between 1 and 8760 hours (1 year)".to_string(),
                }),
            ));
        }
        set_system_setting(
            &state.db,
            "token_expiry_hours",
            &hours.to_string(),
            Some("JWT security token expiration duration in hours"),
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Failed to save token expiry: {}", e),
                }),
            )
        })?;
    }

    if let Some(google_cid) = &payload.google_client_id {
        let trimmed = google_cid.trim();
        set_system_setting(
            &state.db,
            "google_client_id",
            trimmed,
            Some("Google Identity Services OAuth 2.0 Client ID for Google SSO"),
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Failed to save Google Client ID: {}", e),
                }),
            )
        })?;
    }

    if let Some(allowed) = &payload.allowed_ips {
        let trimmed = allowed.trim();
        if !trimmed.is_empty() {
            for raw_entry in trimmed.split(',') {
                let entry = raw_entry.trim();
                if entry.is_empty() {
                    continue;
                }
                let is_valid = if let Some((ip, prefix_str)) = entry.split_once('/') {
                    IpAddr::from_str(ip.trim()).is_ok()
                        && prefix_str
                            .trim()
                            .parse::<u8>()
                            .map(|p| p <= 128)
                            .unwrap_or(false)
                } else {
                    IpAddr::from_str(entry).is_ok()
                };

                if !is_valid {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(AuthErrorResponse {
                            error: format!(
                                "Invalid IP or CIDR network '{}'. Expected format: '192.168.1.100' or '10.0.0.0/8'",
                                entry
                            ),
                        }),
                    ));
                }
            }
        }

        set_system_setting(
            &state.db,
            "superadmin_allowed_ips",
            trimmed,
            Some("Whitelisted client IP addresses and CIDR networks permitted to access the Super Admin console"),
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Failed to save allowed IPs: {}", e),
                }),
            )
        })?;

        crate::superadmin::security::log_security_event(
            &state.db,
            "SUPERADMIN_IP_RESTRICTION_UPDATED",
            &auth.user_id.to_string(),
            serde_json::json!({ "allowed_ips": trimmed }),
            None,
        )
        .await;
    }

    // Return updated settings
    get_platform_settings(auth, State(state)).await
}
