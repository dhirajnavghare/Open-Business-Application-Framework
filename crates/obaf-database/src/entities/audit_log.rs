use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "audit_logs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub action: String, // e.g. "AUTH_LOGIN", "ORDER_CANCELLED", "PAYMENT_RECORDED", "PRESET_BOOTSTRAPPED"
    pub entity_type: String, // e.g. "order", "payment", "user", "organization"
    pub entity_id: Option<String>,
    pub details: String, // JSON details of the event
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
