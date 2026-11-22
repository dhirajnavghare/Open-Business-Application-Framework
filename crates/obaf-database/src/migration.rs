use obaf_core::{ObafError, Result};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Schema};
use tracing::info;

use crate::entities::{
    api_key, audit_log, booking, coupon, credit_note, currency_exchange_rate, customer, order,
    order_item, organization, payment, pos_shift, product, purchase_order, purchase_order_item,
    stock_location, stock_movement, subscription, supplier, system_setting, user, webhook,
};

pub async fn run_migrations(db: &DatabaseConnection) -> Result<()> {
    info!("Running foundational database migrations...");
    let backend = db.get_database_backend();
    let schema = Schema::new(backend);

    // Platform, Tenant, Credentials, Audit, Settings & Webhooks
    create_table_if_not_exists(db, backend, &schema, system_setting::Entity).await?;
    create_table_if_not_exists(db, backend, &schema, organization::Entity).await?;
    create_table_if_not_exists(db, backend, &schema, user::Entity).await?;
    create_table_if_not_exists(db, backend, &schema, api_key::Entity).await?;
    create_table_if_not_exists(db, backend, &schema, customer::Entity).await?;
    create_table_if_not_exists(db, backend, &schema, supplier::Entity).await?;
    create_table_if_not_exists(db, backend, &schema, webhook::Entity).await?;
    create_table_if_not_exists(db, backend, &schema, audit_log::Entity).await?;

    // Catalog & Multi-Location Inventory
    create_table_if_not_exists(db, backend, &schema, product::Entity).await?;
    create_table_if_not_exists(db, backend, &schema, stock_location::Entity).await?;
    create_table_if_not_exists(db, backend, &schema, stock_movement::Entity).await?;

    // Procurement & Purchase Orders
    create_table_if_not_exists(db, backend, &schema, purchase_order::Entity).await?;
    create_table_if_not_exists(db, backend, &schema, purchase_order_item::Entity).await?;

    // Orders & Transactions
    create_table_if_not_exists(db, backend, &schema, order::Entity).await?;
    create_table_if_not_exists(db, backend, &schema, order_item::Entity).await?;

    // Billing, Payments, Credit Notes & Multi-Currency Exchange Rates
    create_table_if_not_exists(db, backend, &schema, payment::Entity).await?;
    create_table_if_not_exists(db, backend, &schema, credit_note::Entity).await?;
    create_table_if_not_exists(db, backend, &schema, currency_exchange_rate::Entity).await?;

    // Promotions & Coupons
    create_table_if_not_exists(db, backend, &schema, coupon::Entity).await?;

    // POS Shifts & Cash Drawer
    create_table_if_not_exists(db, backend, &schema, pos_shift::Entity).await?;

    // Scheduling, Bookings & Subscriptions
    create_table_if_not_exists(db, backend, &schema, booking::Entity).await?;
    create_table_if_not_exists(db, backend, &schema, subscription::Entity).await?;

    info!("Migrations completed successfully");
    Ok(())
}

async fn create_table_if_not_exists<E>(
    db: &DatabaseConnection,
    backend: DbBackend,
    schema: &Schema,
    entity: E,
) -> Result<()>
where
    E: sea_orm::EntityTrait,
{
    let mut stmt = schema.create_table_from_entity(entity);
    stmt.if_not_exists();

    let sql = backend.build(&stmt);
    db.execute(sql).await.map_err(|e| ObafError::Database {
        message: format!("Failed to create table for entity: {}", e),
    })?;

    Ok(())
}
