use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;
use uuid::Uuid;

/// Strongly-typed identifier for an Organization / Tenant boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[repr(transparent)]
pub struct TenantId(pub Uuid);

impl TenantId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for TenantId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for TenantId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<TenantId> for Uuid {
    fn from(t: TenantId) -> Self {
        t.0
    }
}

impl From<Uuid> for BranchId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<BranchId> for Uuid {
    fn from(b: BranchId) -> Self {
        b.0
    }
}

/// Strongly-typed identifier for a Branch or Location within a Tenant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[repr(transparent)]
pub struct BranchId(pub Uuid);

impl BranchId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for BranchId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for BranchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Request context holding trusted tenant context extracted from authentication.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TenantContext {
    pub tenant_id: TenantId,
    pub branch_id: Option<BranchId>,
    pub user_id: Option<Uuid>,
    pub roles: Vec<String>,
}
