use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "stock_movements")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub product_id: Uuid,
    pub movement_type: String, // "RECEIPT", "SALE", "ADJUSTMENT", "RETURN"
    pub quantity: Decimal,     // positive for receipt, negative for sale
    pub batch_number: Option<String>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub reference_id: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
