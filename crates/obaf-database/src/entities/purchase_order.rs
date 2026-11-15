use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_orders")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    #[sea_orm(unique)]
    pub po_number: String,
    pub supplier_id: Uuid,
    pub status: String, // "DRAFT", "SUBMITTED", "PARTIALLY_RECEIVED", "RECEIVED", "CANCELLED"
    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub total_amount: Decimal,
    pub currency: String,
    pub notes: Option<String>,
    pub expected_delivery_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::purchase_order_item::Entity")]
    Items,
    #[sea_orm(
        belongs_to = "super::supplier::Entity",
        from = "Column::SupplierId",
        to = "super::supplier::Column::Id"
    )]
    Supplier,
}

impl Related<super::purchase_order_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Items.def()
    }
}

impl Related<super::supplier::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Supplier.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
