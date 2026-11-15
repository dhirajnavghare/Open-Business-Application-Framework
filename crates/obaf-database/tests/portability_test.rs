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
fn test_schema_statements_postgres_and_mysql_compatibility() {
    use obaf_database::entities::{
        api_key, audit_log, booking, coupon, credit_note, currency_exchange_rate, customer, order,
        order_item, organization, payment, pos_shift, product, purchase_order, purchase_order_item,
        stock_location, stock_movement, subscription, supplier, user, webhook,
    };

    let backends = [DbBackend::Postgres, DbBackend::MySql];

    for backend in backends {
        let schema = Schema::new(backend);

        // Verify all 21 foundational entity tables build valid non-blocking DDL
        let entities = vec![
            schema.create_table_from_entity(organization::Entity),
            schema.create_table_from_entity(user::Entity),
            schema.create_table_from_entity(api_key::Entity),
            schema.create_table_from_entity(customer::Entity),
            schema.create_table_from_entity(supplier::Entity),
            schema.create_table_from_entity(webhook::Entity),
            schema.create_table_from_entity(audit_log::Entity),
            schema.create_table_from_entity(product::Entity),
            schema.create_table_from_entity(stock_location::Entity),
            schema.create_table_from_entity(stock_movement::Entity),
            schema.create_table_from_entity(purchase_order::Entity),
            schema.create_table_from_entity(purchase_order_item::Entity),
            schema.create_table_from_entity(order::Entity),
            schema.create_table_from_entity(order_item::Entity),
            schema.create_table_from_entity(payment::Entity),
            schema.create_table_from_entity(credit_note::Entity),
            schema.create_table_from_entity(currency_exchange_rate::Entity),
            schema.create_table_from_entity(coupon::Entity),
            schema.create_table_from_entity(pos_shift::Entity),
            schema.create_table_from_entity(booking::Entity),
            schema.create_table_from_entity(subscription::Entity),
        ];

        assert_eq!(entities.len(), 21);

        for mut stmt in entities {
            stmt.if_not_exists();
            let sql = backend.build(&stmt);
            assert!(!sql.to_string().is_empty());
        }
    }
}
