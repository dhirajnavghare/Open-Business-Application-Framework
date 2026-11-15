use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use obaf_api::{create_router, AppState};
use obaf_core::{generate_token, TenantId};
use obaf_database::{
    entities::{order, payment, product},
    establish_connection, run_migrations,
};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_order_and_payment_idempotency_keys() {
    let db = establish_connection("sqlite::memory:")
        .await
        .expect("in-memory db");
    run_migrations(&db).await.expect("migrations");

    let tenant_id = Uuid::new_v4();
    let jwt_secret = "test_jwt_secret_1234567890";
    let token =
        generate_token(Uuid::new_v4(), TenantId(tenant_id), "admin", jwt_secret, 24).unwrap();

    let now = chrono::Utc::now();
    // Create a product for ordering
    let product_id = Uuid::new_v4();
    let p = product::ActiveModel {
        id: Set(product_id),
        tenant_id: Set(tenant_id),
        sku: Set("ESPRESSO-01".to_string()),
        name: Set("Artisan Espresso".to_string()),
        category_id: Set(None),
        price: Set(Decimal::from(5)),
        cost_price: Set(Decimal::from(2)),
        tax_rate: Set(Decimal::new(1, 1)), // 0.10 tax
        is_active: Set(true),
        metadata: Set("{}".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };
    p.insert(&db).await.unwrap();

    let state = Arc::new(AppState::new(db.clone(), jwt_secret));
    let app = create_router(state);

    let idempotency_key = "idemp-order-key-9999";
    let order_payload = serde_json::json!({
        "customer_name": "Alice Green",
        "table_or_room": "Table 4",
        "items": [
            {
                "product_id": product_id,
                "quantity": 2
            }
        ]
    });

    // 1. First order request with Idempotency-Key
    let req1 = Request::builder()
        .method("POST")
        .uri("/api/v1/orders")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("idempotency-key", idempotency_key)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(order_payload.to_string()))
        .unwrap();

    let res1 = app.clone().oneshot(req1).await.unwrap();
    assert_eq!(res1.status(), StatusCode::CREATED);
    assert!(res1.headers().get("x-idempotent-replay").is_none());

    let body_bytes1 = axum::body::to_bytes(res1.into_body(), usize::MAX)
        .await
        .unwrap();
    let json1: Value = serde_json::from_slice(&body_bytes1).unwrap();
    let order_id = json1["id"].as_str().unwrap();

    // 2. Duplicate order request with SAME Idempotency-Key (simulating mobile network retry)
    let req2 = Request::builder()
        .method("POST")
        .uri("/api/v1/orders")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("idempotency-key", idempotency_key)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(order_payload.to_string()))
        .unwrap();

    let res2 = app.clone().oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::CREATED);

    // Verify System Design Primer idempotency replay header
    assert_eq!(res2.headers().get("x-idempotent-replay").unwrap(), "true");

    let body_bytes2 = axum::body::to_bytes(res2.into_body(), usize::MAX)
        .await
        .unwrap();
    let json2: Value = serde_json::from_slice(&body_bytes2).unwrap();
    assert_eq!(json2["id"].as_str().unwrap(), order_id);

    // Verify only ONE order was inserted in the database
    let total_orders = order::Entity::find().all(&db).await.unwrap();
    assert_eq!(total_orders.len(), 1);

    // 3. Test payment idempotency key
    let payment_key = "idemp-pay-key-7777";
    let payment_payload = serde_json::json!({
        "order_id": order_id,
        "amount": "11.00",
        "payment_method": "CARD",
        "reference_number": "AUTH_987654"
    });

    let pay_req1 = Request::builder()
        .method("POST")
        .uri("/api/v1/payments")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("idempotency-key", payment_key)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payment_payload.to_string()))
        .unwrap();

    let pay_res1 = app.clone().oneshot(pay_req1).await.unwrap();
    assert_eq!(pay_res1.status(), StatusCode::CREATED);
    assert!(pay_res1.headers().get("x-idempotent-replay").is_none());

    // Duplicate payment request
    let pay_req2 = Request::builder()
        .method("POST")
        .uri("/api/v1/payments")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header("idempotency-key", payment_key)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payment_payload.to_string()))
        .unwrap();

    let pay_res2 = app.oneshot(pay_req2).await.unwrap();
    assert_eq!(pay_res2.status(), StatusCode::CREATED);
    assert_eq!(
        pay_res2.headers().get("x-idempotent-replay").unwrap(),
        "true"
    );

    // Verify only ONE payment was inserted in the database
    let total_payments = payment::Entity::find().all(&db).await.unwrap();
    assert_eq!(total_payments.len(), 1);
}
