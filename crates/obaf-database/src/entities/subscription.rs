use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "subscriptions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub customer_id: Uuid,
    pub plan_name: String, // "Monthly Gold Gym Membership", "Daily Tiffin Plan"
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub status: String, // "ACTIVE", "PAUSED", "EXPIRED", "CANCELLED"
    pub amount: Decimal,
    pub billing_cycle: String, // "MONTHLY", "DAILY", "ANNUAL"
    pub metadata: String, // JSON for domain custom rules (e.g. pause dates, dietary preferences)
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
