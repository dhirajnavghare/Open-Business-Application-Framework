use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use obaf_database::entities::audit_log;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Serialize, ToSchema)]
pub struct AuditLogResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub details: serde_json::Value,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct AuditLogQuery {
    pub action: Option<String>,
    pub entity_type: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/audit-logs",
    tag = "Admin",
    security(("bearer_auth" = [])),
    params(
        AuditLogQuery
    ),
    responses(
        (status = 200, description = "List of immutable audit trail logs", body = Vec<AuditLogResponse>),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn list_audit_logs(
    State(state): State<SharedState>,
    auth: AuthUser,
    Query(query): Query<AuditLogQuery>,
) -> Result<Json<Vec<AuditLogResponse>>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin"])?;

    let mut q = audit_log::Entity::find()
        .filter(audit_log::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .order_by_desc(audit_log::Column::CreatedAt);

    if let Some(act) = query.action {
        q = q.filter(audit_log::Column::Action.eq(act));
    }

    if let Some(ent) = query.entity_type {
        q = q.filter(audit_log::Column::EntityType.eq(ent));
    }

    let limit = query.limit.unwrap_or(50).min(200);
    q = q.limit(limit);

    if let Some(offset) = query.offset {
        q = q.offset(offset);
    }

    let logs = q.all(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Database error: {}", e),
            }),
        )
    })?;

    let resp = logs
        .into_iter()
        .map(|m| {
            let details = serde_json::from_str(&m.details).unwrap_or(serde_json::json!({}));
            AuditLogResponse {
                id: m.id,
                tenant_id: m.tenant_id,
                actor_id: m.actor_id,
                action: m.action,
                entity_type: m.entity_type,
                entity_id: m.entity_id,
                details,
                ip_address: m.ip_address,
                created_at: m.created_at,
            }
        })
        .collect();

    Ok(Json(resp))
}

pub struct NewAuditEntry {
    pub tenant_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub details: serde_json::Value,
    pub ip_address: Option<String>,
}

/// Helper function to asynchronously append an audit trail event.
pub async fn record_audit_log(db: &obaf_database::DatabaseConnection, entry: NewAuditEntry) {
    let new_log = audit_log::ActiveModel {
        id: Set(Uuid::new_v4()),
        tenant_id: Set(entry.tenant_id),
        actor_id: Set(entry.actor_id),
        action: Set(entry.action),
        entity_type: Set(entry.entity_type),
        entity_id: Set(entry.entity_id),
        details: Set(entry.details.to_string()),
        ip_address: Set(entry.ip_address),
        created_at: Set(Utc::now()),
    };

    let _ = new_log.insert(db).await;
}
