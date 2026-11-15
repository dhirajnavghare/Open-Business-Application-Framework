use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use obaf_api::create_router;
use obaf_api::state::AppState;
use obaf_core::security::generate_token;
use obaf_core::tenant::TenantId;
use obaf_database::{migration::run_migrations, pool::establish_connection};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_global_search_and_stock_balance() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    run_migrations(&db).await.unwrap();

    let jwt_secret = "test_secret_32_characters_minimum_len";
    let state = Arc::new(AppState::new(db, jwt_secret));
    let app = create_router(state);

    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let token = generate_token(user_id, TenantId(tenant_id), "admin", jwt_secret, 24).unwrap();

    // 1. Create a Product
    let prod_req = Request::builder()
        .method("POST")
        .uri("/api/v1/catalog/products")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "sku": "ROAST-COFFEE-01",
                "name": "Artisanal Espresso Beans 1kg",
                "price": "32.00",
                "cost_price": "16.00",
                "tax_rate": "0.05"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(prod_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let prod_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let product_id = prod_json["id"].as_str().unwrap().to_string();

    // 2. Create a Customer
    let cust_req = Request::builder()
        .method("POST")
        .uri("/api/v1/customers")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "name": "Barista Leonardo",
                "phone": "+1-555-4321",
                "email": "leonardo@coffeehouse.test"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(cust_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 3. Create a Supplier
    let supp_req = Request::builder()
        .method("POST")
        .uri("/api/v1/suppliers")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "name": "Ethiopian Highlands Coffee Exporters",
                "contact_person": "Abebe Bikila",
                "phone": "+251-11-555-0100"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(supp_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 4. Test Global Search with keyword "Coffee"
    let search_req = Request::builder()
        .method("GET")
        .uri("/api/v1/search?q=coffee")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(search_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let search_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(search_json["total_results"].as_u64().unwrap() >= 3);
    assert_eq!(search_json["products"][0]["sku"], "ROAST-COFFEE-01");
    assert_eq!(search_json["customers"][0]["name"], "Barista Leonardo");
    assert_eq!(
        search_json["suppliers"][0]["name"],
        "Ethiopian Highlands Coffee Exporters"
    );

    // 5. Test Stock Movement & Ledger Balance
    // Inward receipt: +100 units
    let inward_req = Request::builder()
        .method("POST")
        .uri("/api/v1/inventory/movements")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "product_id": product_id,
                "movement_type": "RECEIPT",
                "quantity": "100.00",
                "reference_id": "PO-HIGHLANDS-01"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(inward_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // Sale deduction: -15 units
    let sale_req = Request::builder()
        .method("POST")
        .uri("/api/v1/inventory/movements")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "product_id": product_id,
                "movement_type": "SALE",
                "quantity": "-15.00",
                "reference_id": "ORD-001"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(sale_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // Check calculated stock balance: 100 - 15 = 85.00
    let bal_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/inventory/balance/{}", product_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(bal_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let bal_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let qty_str = bal_json["on_hand_quantity"].as_str().unwrap();
    assert_eq!(
        rust_decimal::Decimal::from_str_exact(qty_str).unwrap(),
        rust_decimal::Decimal::from(85)
    );
    assert_eq!(bal_json["movements_count"], 2);
}
