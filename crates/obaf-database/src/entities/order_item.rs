use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "order_items")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub order_id: Uuid,
    pub product_id: Uuid,
    pub item_name: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub tax_rate: Decimal,
    pub subtotal: Decimal,
    pub notes: Option<String>, // e.g. "No onions, extra spicy"
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::order::Entity",
        from = "Column::OrderId",
        to = "super::order::Column::Id"
    )]
    Order,
}

impl Related<super::order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Order.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
