use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_order_items")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub purchase_order_id: Uuid,
    pub product_id: Uuid,
    pub item_name: String,
    pub quantity_ordered: Decimal,
    pub quantity_received: Decimal,
    pub unit_cost: Decimal,
    pub subtotal: Decimal,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::purchase_order::Entity",
        from = "Column::PurchaseOrderId",
        to = "super::purchase_order::Column::Id"
    )]
    PurchaseOrder,
}

impl Related<super::purchase_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PurchaseOrder.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
