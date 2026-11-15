use obaf_database::migration::run_migrations;
use sea_orm::{Database, DbBackend, Schema};

#[tokio::test]
async fn test_database_migrations_sqlite_in_memory() {
    let db = Database::connect("sqlite::memory:")
        .await
        .expect("Failed to create SQLite in-memory database");

    let result = run_migrations(&db).await;
    assert!(result.is_ok(), "SQLite migrations should execute cleanly");
}

#[test]
fn test_schema_statements_postgres_compatibility() {
    use obaf_database::entities::{
        api_key, audit_log, booking, customer, order, order_item, organization, payment, product,
        stock_movement, subscription, supplier, user, webhook,
    };

    let schema = Schema::new(DbBackend::Postgres);

    // Verify all entity tables build valid Postgres DDL
    let entities = vec![
        schema.create_table_from_entity(organization::Entity),
        schema.create_table_from_entity(user::Entity),
        schema.create_table_from_entity(api_key::Entity),
        schema.create_table_from_entity(customer::Entity),
        schema.create_table_from_entity(supplier::Entity),
        schema.create_table_from_entity(webhook::Entity),
        schema.create_table_from_entity(audit_log::Entity),
        schema.create_table_from_entity(product::Entity),
        schema.create_table_from_entity(stock_movement::Entity),
        schema.create_table_from_entity(order::Entity),
        schema.create_table_from_entity(order_item::Entity),
        schema.create_table_from_entity(payment::Entity),
        schema.create_table_from_entity(booking::Entity),
        schema.create_table_from_entity(subscription::Entity),
    ];

    for mut stmt in entities {
        stmt.if_not_exists();
        let sql = DbBackend::Postgres.build(&stmt);
        assert!(!sql.to_string().is_empty());
    }
}
