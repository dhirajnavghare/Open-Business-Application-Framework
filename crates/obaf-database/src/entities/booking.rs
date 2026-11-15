use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "bookings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub customer_id: Option<Uuid>,
    pub resource_name: String, // "Table #4", "Room 101", "Dr. Slot", "Banquet Hall"
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub status: String, // "CONFIRMED", "CHECKED_IN", "COMPLETED", "CANCELLED"
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
