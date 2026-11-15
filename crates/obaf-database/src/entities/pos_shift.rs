use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "pos_shifts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub cashier_id: Uuid,
    pub register_name: String,
    pub status: String, // "OPEN", "CLOSED"
    pub opening_float: Decimal,
    pub cash_sales: Decimal,
    pub card_sales: Decimal,
    pub upi_sales: Decimal,
    pub cash_refunds: Decimal,
    pub expected_cash: Decimal,
    pub counted_cash: Option<Decimal>,
    pub variance: Option<Decimal>,
    pub notes: Option<String>,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
