use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "payments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub order_id: Option<Uuid>,
    pub customer_id: Option<Uuid>,
    pub amount: Decimal,
    pub payment_method: String, // "CASH", "CARD", "UPI", "BANK_TRANSFER"
    pub reference_number: Option<String>,
    pub status: String, // "SUCCESS", "REFUNDED", "FAILED"
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
