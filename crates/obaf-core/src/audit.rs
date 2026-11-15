use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Immutable audit record metadata attached to administrative and financial changes.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditContext {
    pub actor_id: Option<Uuid>,
    pub actor_role: Option<String>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl Default for AuditContext {
    fn default() -> Self {
        Self {
            actor_id: None,
            actor_role: None,
            client_ip: None,
            user_agent: None,
            timestamp: Utc::now(),
        }
    }
}
