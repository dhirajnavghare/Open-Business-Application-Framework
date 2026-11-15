use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "orders")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    #[sea_orm(unique)]
    pub order_number: String,
    pub customer_name: Option<String>,
    pub table_or_room: Option<String>, // Restaurant Table # or Hotel Room #
    pub status: String, // "DRAFT", "CONFIRMED", "IN_PROGRESS", "READY", "COMPLETED", "CANCELLED"
    pub subtotal: Decimal,
    pub tax_total: Decimal,
    pub discount_total: Decimal,
    pub grand_total: Decimal,
    pub metadata: String, // JSON
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::order_item::Entity")]
    Items,
}

impl Related<super::order_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Items.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
