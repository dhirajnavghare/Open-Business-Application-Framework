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

use axum::{
    extract::State,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use obaf_core::verify_token;
use obaf_database::entities::{api_key, organization};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::state::SharedState;

/// Map API request path to its corresponding modular engine name.
pub fn extract_gated_module(path: &str) -> Option<&'static str> {
    if path.starts_with("/api/v1/catalog") {
        Some("catalog")
    } else if path.starts_with("/api/v1/inventory") {
        Some("inventory")
    } else if path.starts_with("/api/v1/orders") {
        Some("orders")
    } else if path.starts_with("/api/v1/billing") {
        Some("billing")
    } else if path.starts_with("/api/v1/payments") {
        Some("payments")
    } else if path.starts_with("/api/v1/customers") {
        Some("customers")
    } else if path.starts_with("/api/v1/suppliers") {
        Some("suppliers")
    } else if path.starts_with("/api/v1/procurement") {
        Some("procurement")
    } else if path.starts_with("/api/v1/pos") {
        Some("pos")
    } else if path.starts_with("/api/v1/bookings") {
        Some("bookings")
    } else if path.starts_with("/api/v1/subscriptions") {
        Some("subscriptions")
    } else if path.starts_with("/api/v1/promotions") {
        Some("promotions")
    } else if path.starts_with("/api/v1/reports") {
        Some("reports")
    } else if path.starts_with("/api/v1/webhooks") {
        Some("webhooks")
    } else if path.starts_with("/api/v1/sync") {
        Some("sync")
    } else {
        None
    }
}

/// Middleware that dynamically enforces per-organization API module enablement.
/// If an organization has not enabled a particular module (e.g. `bookings` or `pos`),
/// tenant requests targeting that module are rejected with 403 Forbidden.
/// The Platform Super Administrator bypasses this check.
pub async fn module_gating_middleware(
    State(state): State<SharedState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let path = request.uri().path();

    // Check if caller is authenticated as Super Admin
    let is_superadmin = if let Some(auth_header) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            if let Ok(claims) = verify_token(token, &state.jwt_secret) {
                claims.role == "superadmin" && !state.is_token_revoked(token)
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    // Platform Super Admin bypasses maintenance mode and emergency locks
    if is_superadmin {
        return next.run(request).await;
    }

    // Enforce Global Maintenance Mode & Emergency Lockdown on tenant APIs
    if path.starts_with("/api/v1/") && !path.starts_with("/api/v1/superadmin") {
        // 1. Maintenance Mode Check
        if let Ok(Some(m_mode)) =
            obaf_database::get_system_setting(&state.db, "maintenance_mode").await
        {
            if m_mode == "true" {
                let msg = obaf_database::get_system_setting(&state.db, "maintenance_message")
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| {
                        "Platform is undergoing scheduled maintenance. Please check back shortly."
                            .to_string()
                    });
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({
                        "error": "MAINTENANCE_MODE",
                        "message": msg
                    })),
                )
                    .into_response();
            }
        }

        // 2. Emergency Global Mutation Lockdown Check
        if matches!(
            *request.method(),
            axum::http::Method::POST
                | axum::http::Method::PUT
                | axum::http::Method::PATCH
                | axum::http::Method::DELETE
        ) {
            if let Ok(Some(lockdown)) =
                obaf_database::get_system_setting(&state.db, "emergency_lockdown").await
            {
                if lockdown == "true" {
                    let reason = obaf_database::get_system_setting(&state.db, "lockdown_reason")
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "Emergency write freeze active.".to_string());
                    return (
                        StatusCode::LOCKED,
                        Json(serde_json::json!({
                            "error": "EMERGENCY_LOCKDOWN",
                            "message": reason
                        })),
                    )
                        .into_response();
                }
            }
        }
    }

    let module_name = match extract_gated_module(path) {
        Some(m) => m,
        None => return next.run(request).await,
    };

    // 1. Try to extract tenant ID from Bearer JWT
    let mut resolved_tenant_id: Option<Uuid> = None;

    if let Some(auth_header) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            if let Ok(claims) = verify_token(token, &state.jwt_secret) {
                if state.is_token_revoked(token) {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(serde_json::json!({
                            "error": "UNAUTHORIZED",
                            "message": "Authentication token has been revoked."
                        })),
                    )
                        .into_response();
                }
                // Platform Super Admin possesses universal bypass
                if claims.role == "superadmin" {
                    return next.run(request).await;
                }
                resolved_tenant_id = Some(claims.tenant_id.0);
            }
        }
    }

    // 2. Fall back to X-API-Key machine auth if no valid JWT found
    if resolved_tenant_id.is_none() {
        if let Some(api_key_header) = request
            .headers()
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
        {
            let mut hasher = Sha256::new();
            hasher.update(api_key_header.trim().as_bytes());
            let hash = hex::encode(hasher.finalize());

            if let Ok(Some(k)) = api_key::Entity::find()
                .filter(api_key::Column::KeyHash.eq(hash))
                .filter(api_key::Column::IsActive.eq(true))
                .one(state.reader())
                .await
            {
                resolved_tenant_id = Some(k.tenant_id);
            }
        }
    }

    // If caller has no tenant context, forward downstream to let AuthUser reject with 401
    let tenant_id = match resolved_tenant_id {
        Some(t) => t,
        None => return next.run(request).await,
    };

    // 3. Retrieve tenant active modules from fast TTL cache or fallback to database
    let cache_key = format!("tenant_modules:{}", tenant_id);
    let active_modules_json = if let Some(cached) = state.cache.get(&cache_key) {
        cached
    } else {
        match organization::Entity::find_by_id(tenant_id)
            .one(state.reader())
            .await
        {
            Ok(Some(org)) => {
                let json = org.active_modules.clone();
                state.cache.insert(cache_key, json.clone());
                json
            }
            _ => return next.run(request).await,
        }
    };

    let active_modules: Vec<String> =
        serde_json::from_str(&active_modules_json).unwrap_or_default();

    // 4. Verify whether the requested module is enabled
    let is_enabled = active_modules.iter().any(|m| m == "*" || m == module_name);

    if !is_enabled {
        let body = serde_json::json!({
            "error": "Module Disabled",
            "code": "MODULE_DISABLED",
            "module": module_name,
            "message": format!(
                "The '{}' API module is not enabled for your organization. Please contact your system super administrator to enable it.",
                module_name
            )
        });
        let mut response = Json(body).into_response();
        *response.status_mut() = StatusCode::FORBIDDEN;
        return response;
    }

    next.run(request).await
}
