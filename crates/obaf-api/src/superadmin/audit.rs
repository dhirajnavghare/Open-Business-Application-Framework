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
use chrono::{DateTime, Utc};
use obaf_database::entities::audit_log;
use sea_orm::{EntityTrait, QueryOrder, QuerySelect};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditLogEntryResponse {
    pub id: Uuid,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub details: serde_json::Value,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[utoipa::path(
    get,
    path = "/api/v1/superadmin/audit-logs",
    tag = "SuperAdmin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Recent platform security audit logs", body = Vec<AuditLogEntryResponse>),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse),
        (status = 403, description = "Forbidden: superadmin required", body = AuthErrorResponse)
    )
)]
pub async fn list_audit_logs(
    auth: AuthUser,
    State(state): State<SharedState>,
) -> Result<Json<Vec<AuditLogEntryResponse>>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_superadmin()?;

    let logs = audit_log::Entity::find()
        .order_by_desc(audit_log::Column::CreatedAt)
        .limit(100)
        .all(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Failed to fetch audit logs: {}", e),
                }),
            )
        })?;

    let responses: Vec<AuditLogEntryResponse> = logs
        .into_iter()
        .map(|m| {
            let details_val = serde_json::from_str(&m.details).unwrap_or(serde_json::json!({
                "raw": m.details
            }));
            AuditLogEntryResponse {
                id: m.id,
                action: m.action,
                entity_type: m.entity_type,
                entity_id: m.entity_id,
                details: details_val,
                ip_address: m.ip_address,
                created_at: m.created_at,
            }
        })
        .collect();

    Ok(Json(responses))
}
