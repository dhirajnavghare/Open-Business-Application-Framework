use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::tenant::TenantId;

/// Envelope for versioned domain events published to in-memory buses or outboxes.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DomainEvent<T> {
    pub id: Uuid,
    pub event_type: String,
    pub schema_version: u32,
    pub tenant_id: TenantId,
    pub timestamp: DateTime<Utc>,
    pub payload: T,
}

impl<T> DomainEvent<T> {
    pub fn new(event_type: impl Into<String>, tenant_id: TenantId, payload: T) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: event_type.into(),
            schema_version: 1,
            tenant_id,
            timestamp: Utc::now(),
            payload,
        }
    }
}
