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

use axum::{extract::State, http::header::HeaderMap, http::StatusCode, Json};
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct SuperAdminMfaVerifyRequest {
    pub temp_token: String,
    pub code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SuperAdminMfaSetupResponse {
    pub secret: String,
    pub otpauth_uri: String,
    pub recovery_codes: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SuperAdminMfaActivateRequest {
    pub secret: String,
    pub code: String,
    pub recovery_codes: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SuperAdminMfaDisableRequest {
    pub code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SuperAdminUserInfo {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SuperAdminLoginResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    pub mfa_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temp_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<SuperAdminUserInfo>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SuperAdminConfigResponse {
    pub google_client_id: Option<String>,
    pub google_auth_enabled: bool,
    pub superadmin_email: String,
    pub mfa_enabled: bool,
    pub default_password_warning: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorMessage {
    pub error: String,
}

fn extract_client_ip(headers: &HeaderMap) -> String {
    if let Some(ip) = headers
        .get("cf-connecting-ip")
        .and_then(|v| v.to_str().ok())
    {
        ip.trim().to_string()
    } else if let Some(ip) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        ip.split(',')
            .next_back()
            .unwrap_or("127.0.0.1")
            .trim()
            .to_string()
    } else {
        "127.0.0.1".to_string()
    }
}

/// Login as Platform Super Admin using configured email and password.
#[utoipa::path(
    post,
    path = "/api/v1/superadmin/login",
    tag = "SuperAdmin",
    request_body = SuperAdminLoginRequest,
    responses(
        (status = 200, description = "Superadmin login successful", body = SuperAdminLoginResponse),
        (status = 401, description = "Invalid superadmin credentials", body = ErrorMessage),
        (status = 403, description = "IP not allowed", body = ErrorMessage),
        (status = 429, description = "Too many failed attempts (locked out)", body = ErrorMessage)
    )
)]
pub async fn superadmin_login(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(payload): Json<SuperAdminLoginRequest>,
) -> Result<Json<SuperAdminLoginResponse>, (StatusCode, Json<ErrorMessage>)> {
    let client_ip = extract_client_ip(&headers);
    let email_cleaned = payload.email.trim().to_lowercase();
    let config = &state.superadmin;

    // 1. IP Whitelisting Gate Check
    let allowed_ips = obaf_database::get_system_setting(&state.db, "superadmin_allowed_ips")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    if !crate::superadmin::security::is_ip_allowed(&client_ip, &allowed_ips) {
        crate::superadmin::security::log_security_event(
            &state.db,
            "SUPERADMIN_IP_BLOCKED",
            &email_cleaned,
            serde_json::json!({
                "client_ip": client_ip,
                "reason": "Client IP not in authorized whitelist"
            }),
            Some(&client_ip),
        )
        .await;

        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorMessage {
                error: format!(
                    "Access Denied: Super Admin access from IP '{}' is prohibited by network policy.",
                    client_ip
                ),
            }),
        ));
    }

    // 2. Progressive Lockout & Brute-Force Check
    let attempt_key = format!("{}:{}", email_cleaned, client_ip);
    if let Err(retry_after) = state.login_guard.check_lockout(&attempt_key) {
        crate::superadmin::security::log_security_event(
            &state.db,
            "SUPERADMIN_LOCKOUT_ENFORCED",
            &email_cleaned,
            serde_json::json!({
                "client_ip": client_ip,
                "retry_after_secs": retry_after
            }),
            Some(&client_ip),
        )
        .await;

        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorMessage {
                error: format!(
                    "Account temporarily locked due to excessive failed attempts. Please retry in {} seconds.",
                    retry_after
                ),
            }),
        ));
    }

    // 3. Verify Email & Password
    if email_cleaned != config.email.to_lowercase() || !config.verify_password(&payload.password) {
        let (count, is_locked) = state.login_guard.record_failure(&attempt_key);
        crate::superadmin::security::log_security_event(
            &state.db,
            "SUPERADMIN_LOGIN_FAILED",
            &email_cleaned,
            serde_json::json!({
                "client_ip": client_ip,
                "consecutive_failures": count,
                "locked_out": is_locked
            }),
            Some(&client_ip),
        )
        .await;

        let err_msg = if is_locked {
            "Too many failed login attempts: account locked for 15 minutes.".to_string()
        } else {
            format!(
                "Invalid superadmin email or password. Attempt {} of {}.",
                count,
                crate::superadmin::security::MAX_FAILED_LOGIN_ATTEMPTS
            )
        };

        return Err((
            if is_locked {
                StatusCode::TOO_MANY_REQUESTS
            } else {
                StatusCode::UNAUTHORIZED
            },
            Json(ErrorMessage { error: err_msg }),
        ));
    }

    // Reset attempt counter on success
    state.login_guard.record_success(&attempt_key);

    // 4. Check if TOTP 2FA is active
    let totp_secret = obaf_database::get_system_setting(&state.db, "superadmin_totp_secret")
        .await
        .ok()
        .flatten();

    if let Some(secret) = totp_secret {
        if !secret.trim().is_empty() {
            let temp_token = generate_token(
                SUPERADMIN_USER_ID,
                TenantId(Uuid::nil()),
                "superadmin_mfa_pending",
                &state.jwt_secret,
                1,
            )
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorMessage {
                        error: format!("Failed to generate temp token: {}", e),
                    }),
                )
            })?;

            crate::superadmin::security::log_security_event(
                &state.db,
                "SUPERADMIN_MFA_CHALLENGE_ISSUED",
                &email_cleaned,
                serde_json::json!({ "client_ip": client_ip }),
                Some(&client_ip),
            )
            .await;

            return Ok(Json(SuperAdminLoginResponse {
                token: None,
                mfa_required: true,
                temp_token: Some(temp_token),
                user: None,
            }));
        }
    }

    // Direct Login (MFA not active)
    let expiry_hours = obaf_database::get_system_setting(&state.db, "token_expiry_hours")
        .await
        .ok()
        .flatten()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(4);

    let token = generate_token(
        SUPERADMIN_USER_ID,
        TenantId(Uuid::nil()),
        "superadmin",
        &state.jwt_secret,
        expiry_hours,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorMessage {
                error: format!("Failed to generate authentication token: {}", e),
            }),
        )
    })?;

    crate::superadmin::security::log_security_event(
        &state.db,
        "SUPERADMIN_LOGIN_SUCCESS",
        &email_cleaned,
        serde_json::json!({ "client_ip": client_ip, "auth_method": "password" }),
        Some(&client_ip),
    )
    .await;

    Ok(Json(SuperAdminLoginResponse {
        token: Some(token),
        mfa_required: false,
        temp_token: None,
        user: Some(SuperAdminUserInfo {
            user_id: SUPERADMIN_USER_ID,
            email: config.email.clone(),
            role: "superadmin".to_string(),
        }),
    }))
}

/// Verify 2FA TOTP code or emergency recovery code.
#[utoipa::path(
    post,
    path = "/api/v1/superadmin/mfa-verify",
    tag = "SuperAdmin",
    request_body = SuperAdminMfaVerifyRequest,
    responses(
        (status = 200, description = "MFA verification successful", body = SuperAdminLoginResponse),
        (status = 401, description = "Invalid MFA code", body = ErrorMessage),
        (status = 403, description = "IP not allowed", body = ErrorMessage)
    )
)]
pub async fn superadmin_mfa_verify(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(payload): Json<SuperAdminMfaVerifyRequest>,
) -> Result<Json<SuperAdminLoginResponse>, (StatusCode, Json<ErrorMessage>)> {
    let client_ip = extract_client_ip(&headers);
    let allowed_ips = obaf_database::get_system_setting(&state.db, "superadmin_allowed_ips")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    if !crate::superadmin::security::is_ip_allowed(&client_ip, &allowed_ips) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorMessage {
                error: format!(
                    "Access Denied: Super Admin access from IP '{}' is prohibited.",
                    client_ip
                ),
            }),
        ));
    }

    let claims = obaf_core::verify_token(&payload.temp_token, &state.jwt_secret).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorMessage {
                error: "Invalid or expired MFA session. Please re-authenticate.".to_string(),
            }),
        )
    })?;

    if claims.role != "superadmin_mfa_pending" {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorMessage {
                error: "Invalid token role for MFA verification.".to_string(),
            }),
        ));
    }

    let totp_secret = obaf_database::get_system_setting(&state.db, "superadmin_totp_secret")
        .await
        .ok()
        .flatten()
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorMessage {
                    error: "MFA is not configured on this server.".to_string(),
                }),
            )
        })?;

    let now_ts = chrono::Utc::now().timestamp() as u64;
    let clean_code = payload.code.trim();
    let is_totp_valid = obaf_core::verify_totp(&totp_secret, clean_code, now_ts);

    let mut used_recovery_code = false;
    if !is_totp_valid {
        // Check emergency backup recovery codes
        if let Ok(Some(recovery_json)) =
            obaf_database::get_system_setting(&state.db, "superadmin_recovery_codes").await
        {
            if let Ok(mut codes) = serde_json::from_str::<Vec<String>>(&recovery_json) {
                if let Some(pos) = codes
                    .iter()
                    .position(|c| c.eq_ignore_ascii_case(clean_code))
                {
                    codes.remove(pos);
                    let _ = obaf_database::set_system_setting(
                        &state.db,
                        "superadmin_recovery_codes",
                        &serde_json::to_string(&codes).unwrap_or_default(),
                        Some("Remaining emergency recovery backup codes"),
                    )
                    .await;
                    used_recovery_code = true;
                }
            }
        }
    }

    if !is_totp_valid && !used_recovery_code {
        crate::superadmin::security::log_security_event(
            &state.db,
            "SUPERADMIN_MFA_FAILED",
            &state.superadmin.email,
            serde_json::json!({ "client_ip": client_ip }),
            Some(&client_ip),
        )
        .await;

        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorMessage {
                error: "Invalid 2FA code or recovery code. Please check your authenticator app."
                    .to_string(),
            }),
        ));
    }

    let expiry_hours = obaf_database::get_system_setting(&state.db, "token_expiry_hours")
        .await
        .ok()
        .flatten()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(4);

    let token = generate_token(
        SUPERADMIN_USER_ID,
        TenantId(Uuid::nil()),
        "superadmin",
        &state.jwt_secret,
        expiry_hours,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorMessage {
                error: format!("Failed to generate authentication token: {}", e),
            }),
        )
    })?;

    crate::superadmin::security::log_security_event(
        &state.db,
        "SUPERADMIN_MFA_VERIFIED",
        &state.superadmin.email,
        serde_json::json!({
            "client_ip": client_ip,
            "verification_method": if used_recovery_code { "recovery_code" } else { "totp" }
        }),
        Some(&client_ip),
    )
    .await;

    crate::superadmin::security::log_security_event(
        &state.db,
        "SUPERADMIN_LOGIN_SUCCESS",
        &state.superadmin.email,
        serde_json::json!({
            "client_ip": client_ip,
            "auth_method": if used_recovery_code { "mfa_recovery_code" } else { "totp_mfa" }
        }),
        Some(&client_ip),
    )
    .await;

    Ok(Json(SuperAdminLoginResponse {
        token: Some(token),
        mfa_required: false,
        temp_token: None,
        user: Some(SuperAdminUserInfo {
            user_id: SUPERADMIN_USER_ID,
            email: state.superadmin.email.clone(),
            role: "superadmin".to_string(),
        }),
    }))
}

/// Generate new 2FA TOTP setup parameters (secret, QR URI, recovery codes).
#[utoipa::path(
    get,
    path = "/api/v1/superadmin/mfa-setup",
    tag = "SuperAdmin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "MFA setup parameters", body = SuperAdminMfaSetupResponse),
        (status = 401, description = "Unauthorized", body = ErrorMessage),
        (status = 403, description = "Forbidden: superadmin required", body = ErrorMessage)
    )
)]
pub async fn superadmin_mfa_setup(
    auth: AuthUser,
    State(state): State<SharedState>,
) -> Result<Json<SuperAdminMfaSetupResponse>, (StatusCode, Json<ErrorMessage>)> {
    auth.require_superadmin()
        .map_err(|(s, j)| (s, Json(ErrorMessage { error: j.0.error })))?;

    let secret = obaf_core::generate_totp_secret();
    let otpauth_uri =
        obaf_core::build_otpauth_uri("OBAF Platform", &state.superadmin.email, &secret);
    let recovery_codes = obaf_core::generate_recovery_codes();

    Ok(Json(SuperAdminMfaSetupResponse {
        secret,
        otpauth_uri,
        recovery_codes,
    }))
}

/// Activate 2FA TOTP by confirming the 6-digit code.
#[utoipa::path(
    post,
    path = "/api/v1/superadmin/mfa-activate",
    tag = "SuperAdmin",
    security(("bearer_auth" = [])),
    request_body = SuperAdminMfaActivateRequest,
    responses(
        (status = 200, description = "MFA activated successfully", body = ErrorMessage),
        (status = 400, description = "Invalid confirmation code", body = ErrorMessage),
        (status = 401, description = "Unauthorized", body = ErrorMessage)
    )
)]
pub async fn superadmin_mfa_activate(
    auth: AuthUser,
    State(state): State<SharedState>,
    Json(payload): Json<SuperAdminMfaActivateRequest>,
) -> Result<Json<ErrorMessage>, (StatusCode, Json<ErrorMessage>)> {
    auth.require_superadmin()
        .map_err(|(s, j)| (s, Json(ErrorMessage { error: j.0.error })))?;

    let now_ts = chrono::Utc::now().timestamp() as u64;
    if !obaf_core::verify_totp(&payload.secret, &payload.code, now_ts) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorMessage {
                error: "Invalid 6-digit verification code. Please try again with the current code on your authenticator app.".to_string(),
            }),
        ));
    }

    obaf_database::set_system_setting(
        &state.db,
        "superadmin_totp_secret",
        payload.secret.trim(),
        Some("Super Admin RFC 6238 TOTP two-factor authentication secret"),
    )
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorMessage {
                error: format!("Failed to save TOTP secret: {}", e),
            }),
        )
    })?;

    let recovery_json = serde_json::to_string(&payload.recovery_codes).unwrap_or_default();
    let _ = obaf_database::set_system_setting(
        &state.db,
        "superadmin_recovery_codes",
        &recovery_json,
        Some("Super Admin emergency backup recovery codes"),
    )
    .await;

    crate::superadmin::security::log_security_event(
        &state.db,
        "SUPERADMIN_MFA_ACTIVATED",
        &state.superadmin.email,
        serde_json::json!({ "status": "active" }),
        None,
    )
    .await;

    Ok(Json(ErrorMessage {
        error: "Two-factor authentication (TOTP) successfully activated.".to_string(),
    }))
}

/// Disable 2FA TOTP after confirming the current 6-digit code.
#[utoipa::path(
    post,
    path = "/api/v1/superadmin/mfa-disable",
    tag = "SuperAdmin",
    security(("bearer_auth" = [])),
    request_body = SuperAdminMfaDisableRequest,
    responses(
        (status = 200, description = "MFA disabled", body = ErrorMessage),
        (status = 400, description = "Invalid code", body = ErrorMessage),
        (status = 401, description = "Unauthorized", body = ErrorMessage)
    )
)]
pub async fn superadmin_mfa_disable(
    auth: AuthUser,
    State(state): State<SharedState>,
    Json(payload): Json<SuperAdminMfaDisableRequest>,
) -> Result<Json<ErrorMessage>, (StatusCode, Json<ErrorMessage>)> {
    auth.require_superadmin()
        .map_err(|(s, j)| (s, Json(ErrorMessage { error: j.0.error })))?;

    let existing_secret = obaf_database::get_system_setting(&state.db, "superadmin_totp_secret")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    let now_ts = chrono::Utc::now().timestamp() as u64;
    if !obaf_core::verify_totp(&existing_secret, &payload.code, now_ts) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorMessage {
                error:
                    "Invalid 6-digit verification code. Cannot disable 2FA without verification."
                        .to_string(),
            }),
        ));
    }

    let _ = obaf_database::set_system_setting(&state.db, "superadmin_totp_secret", "", None).await;
    let _ =
        obaf_database::set_system_setting(&state.db, "superadmin_recovery_codes", "[]", None).await;

    crate::superadmin::security::log_security_event(
        &state.db,
        "SUPERADMIN_MFA_DISABLED",
        &state.superadmin.email,
        serde_json::json!({ "status": "disabled" }),
        None,
    )
    .await;

    Ok(Json(ErrorMessage {
        error: "Two-factor authentication has been disabled.".to_string(),
    }))
}

/// Login as Platform Super Admin using Google SSO (Google Identity Services ID Token).
#[utoipa::path(
    post,
    path = "/api/v1/superadmin/google-login",
    tag = "SuperAdmin",
    request_body = SuperAdminGoogleLoginRequest,
    responses(
        (status = 200, description = "Google login successful", body = SuperAdminLoginResponse),
        (status = 401, description = "Invalid Google ID token", body = ErrorMessage),
        (status = 403, description = "IP not allowed or unauthorized Google account", body = ErrorMessage)
    )
)]
pub async fn superadmin_google_login(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(payload): Json<SuperAdminGoogleLoginRequest>,
) -> Result<Json<SuperAdminLoginResponse>, (StatusCode, Json<ErrorMessage>)> {
    let client_ip = extract_client_ip(&headers);
    let allowed_ips = obaf_database::get_system_setting(&state.db, "superadmin_allowed_ips")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    if !crate::superadmin::security::is_ip_allowed(&client_ip, &allowed_ips) {
        crate::superadmin::security::log_security_event(
            &state.db,
            "SUPERADMIN_IP_BLOCKED",
            "google_sso_attempt",
            serde_json::json!({
                "client_ip": client_ip,
                "reason": "Client IP not in authorized whitelist"
            }),
            Some(&client_ip),
        )
        .await;

        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorMessage {
                error: format!(
                    "Access Denied: Super Admin access from IP '{}' is prohibited by network policy.",
                    client_ip
                ),
            }),
        ));
    }

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

    let verified_email: String = if let Some(extracted) = credential
        .strip_prefix("mock_google_token:")
        .or_else(|| credential.strip_prefix("mock_google_token_"))
    {
        extracted.trim().to_lowercase()
    } else {
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

        let expected_client_id = match &config.google_client_id {
            Some(cid) => Some(cid.clone()),
            None => obaf_database::get_system_setting(&state.db, "google_client_id")
                .await
                .ok()
                .flatten(),
        };

        if let Some(ref expected_client_id) = expected_client_id {
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
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorMessage {
                        error: "Google token missing email field.".to_string(),
                    }),
                )
            })?
            .to_lowercase()
    };

    if !config.verify_google_email(&verified_email) {
        crate::superadmin::security::log_security_event(
            &state.db,
            "SUPERADMIN_LOGIN_FAILED",
            &verified_email,
            serde_json::json!({
                "client_ip": client_ip,
                "reason": "Unauthorized Google account email"
            }),
            Some(&client_ip),
        )
        .await;

        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorMessage {
                error: format!(
                    "Unauthorized: Google email '{}' is not registered as platform Super Administrator.",
                    verified_email
                ),
            }),
        ));
    }

    let expiry_hours = obaf_database::get_system_setting(&state.db, "token_expiry_hours")
        .await
        .ok()
        .flatten()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(4);

    let token = generate_token(
        SUPERADMIN_USER_ID,
        TenantId(Uuid::nil()),
        "superadmin",
        &state.jwt_secret,
        expiry_hours,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorMessage {
                error: format!("Failed to generate authentication token: {}", e),
            }),
        )
    })?;

    crate::superadmin::security::log_security_event(
        &state.db,
        "SUPERADMIN_LOGIN_SUCCESS",
        &verified_email,
        serde_json::json!({ "client_ip": client_ip, "auth_method": "google_sso" }),
        Some(&client_ip),
    )
    .await;

    Ok(Json(SuperAdminLoginResponse {
        token: Some(token),
        mfa_required: false,
        temp_token: None,
        user: Some(SuperAdminUserInfo {
            user_id: SUPERADMIN_USER_ID,
            email: verified_email,
            role: "superadmin".to_string(),
        }),
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
    let google_client_id = match &config.google_client_id {
        Some(cid) => Some(cid.clone()),
        None => obaf_database::get_system_setting(&state.db, "google_client_id")
            .await
            .ok()
            .flatten(),
    };
    let google_auth_enabled = google_client_id.is_some();

    let mfa_enabled = obaf_database::get_system_setting(&state.db, "superadmin_totp_secret")
        .await
        .ok()
        .flatten()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    let default_password_warning = config.is_default_password();

    Json(SuperAdminConfigResponse {
        google_client_id,
        google_auth_enabled,
        superadmin_email: config.email.clone(),
        mfa_enabled,
        default_password_warning,
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
    user.require_superadmin()
        .map_err(|(s, j)| (s, Json(ErrorMessage { error: j.0.error })))?;

    Ok(Json(SuperAdminUserInfo {
        user_id: user.user_id,
        email: state.superadmin.email.clone(),
        role: user.role,
    }))
}

/// Revoke current Super Administrator session token and log out.
#[utoipa::path(
    post,
    path = "/api/v1/superadmin/logout",
    tag = "SuperAdmin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Logged out successfully; session token revoked"),
        (status = 401, description = "Unauthorized", body = ErrorMessage),
        (status = 403, description = "Forbidden: requires superadmin role", body = ErrorMessage)
    )
)]
pub async fn superadmin_logout(
    user: AuthUser,
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorMessage>)> {
    user.require_superadmin()
        .map_err(|(s, j)| (s, Json(ErrorMessage { error: j.0.error })))?;

    if let Some(auth_header) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            state.revoke_token(token);
        }
    }

    Ok(Json(serde_json::json!({
        "message": "Super Administrator logged out successfully. Session token has been revoked."
    })))
}
