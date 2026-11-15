use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use obaf_api::{create_router, AppState};
use obaf_core::{generate_token, TenantId};
use obaf_database::{establish_connection, run_migrations};
use rust_decimal::Decimal;
use serde_json::Value;
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_procurement_po_lifecycle_and_stock_receipt() {
    let db = establish_connection("sqlite::memory:")
        .await
        .expect("in-memory db");
    run_migrations(&db).await.expect("migrations");

    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let jwt_secret = "secret_procurement_test_123456";
    let token = generate_token(user_id, TenantId(tenant_id), "manager", jwt_secret, 24).unwrap();

    let state = Arc::new(AppState::new(db.clone(), jwt_secret));
    let app = create_router(state);

    // 1. Create a Supplier
    let supp_req = Request::builder()
        .method("POST")
        .uri("/api/v1/suppliers")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "name": "Global Tech Distributors",
                "contact_person": "Alice Smith",
                "email": "alice@globaltech.com",
                "phone": "+1-555-888-9999",
                "tax_id": "TAX-US-9912",
                "address": "100 Logistics Way, Chicago, IL"
            })
            .to_string(),
        ))
        .unwrap();

    let supp_res = app.clone().oneshot(supp_req).await.unwrap();
    assert_eq!(supp_res.status(), StatusCode::CREATED);
    let supp_bytes = axum::body::to_bytes(supp_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let supp_json: Value = serde_json::from_slice(&supp_bytes).unwrap();
    let supplier_id = supp_json["id"].as_str().unwrap();

    // 2. Create a Product
    let prod_req = Request::builder()
        .method("POST")
        .uri("/api/v1/catalog/products")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "name": "Industrial Router X500",
                "sku": "ROUTER-X500",
                "price": "249.99",
                "cost_price": "120.00",
                "tax_rate": "0.10"
            })
            .to_string(),
        ))
        .unwrap();

    let prod_res = app.clone().oneshot(prod_req).await.unwrap();
    assert_eq!(prod_res.status(), StatusCode::CREATED);
    let prod_bytes = axum::body::to_bytes(prod_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let prod_json: Value = serde_json::from_slice(&prod_bytes).unwrap();
    let product_id = prod_json["id"].as_str().unwrap();

    // Verify initial stock balance is 0
    let bal_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/inventory/balance/{}", product_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let bal_res = app.clone().oneshot(bal_req).await.unwrap();
    assert_eq!(bal_res.status(), StatusCode::OK);
    let bal_bytes = axum::body::to_bytes(bal_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let bal_json: Value = serde_json::from_slice(&bal_bytes).unwrap();
    assert_eq!(
        bal_json["on_hand_quantity"]
            .as_str()
            .unwrap()
            .parse::<Decimal>()
            .unwrap(),
        Decimal::ZERO
    );

    // 3. Create a Purchase Order
    let po_req = Request::builder()
        .method("POST")
        .uri("/api/v1/procurement/purchase-orders")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "supplier_id": supplier_id,
                "currency": "USD",
                "tax_rate": "0.10",
                "notes": "Urgent Q4 inventory restocking",
                "items": [
                    {
                        "product_id": product_id,
                        "item_name": "Industrial Router X500",
                        "quantity_ordered": "50.00",
                        "unit_cost": "120.00"
                    }
                ]
            })
            .to_string(),
        ))
        .unwrap();

    let po_res = app.clone().oneshot(po_req).await.unwrap();
    assert_eq!(po_res.status(), StatusCode::CREATED);
    let po_bytes = axum::body::to_bytes(po_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let po_json: Value = serde_json::from_slice(&po_bytes).unwrap();
    let po_id = po_json["id"].as_str().unwrap();
    assert_eq!(po_json["status"], "SUBMITTED");
    assert_eq!(
        po_json["subtotal"]
            .as_str()
            .unwrap()
            .parse::<Decimal>()
            .unwrap(),
        Decimal::from(6000)
    );
    assert_eq!(
        po_json["tax_amount"]
            .as_str()
            .unwrap()
            .parse::<Decimal>()
            .unwrap(),
        Decimal::from(600)
    );
    assert_eq!(
        po_json["total_amount"]
            .as_str()
            .unwrap()
            .parse::<Decimal>()
            .unwrap(),
        Decimal::from(6600)
    );

    let po_item_id = po_json["items"][0]["id"].as_str().unwrap();

    // 4. List Purchase Orders
    let list_po_req = Request::builder()
        .method("GET")
        .uri("/api/v1/procurement/purchase-orders")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let list_po_res = app.clone().oneshot(list_po_req).await.unwrap();
    assert_eq!(list_po_res.status(), StatusCode::OK);

    // 5. Partially Receive PO (20 units out of 50)
    let rec1_req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/v1/procurement/purchase-orders/{}/receive",
            po_id
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "items": [
                    {
                        "item_id": po_item_id,
                        "quantity_received": "20.00",
                        "batch_number": "BATCH-2026-A1"
                    }
                ]
            })
            .to_string(),
        ))
        .unwrap();

    let rec1_res = app.clone().oneshot(rec1_req).await.unwrap();
    assert_eq!(rec1_res.status(), StatusCode::OK);
    let rec1_bytes = axum::body::to_bytes(rec1_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let rec1_json: Value = serde_json::from_slice(&rec1_bytes).unwrap();
    assert_eq!(rec1_json["status"], "PARTIALLY_RECEIVED");
    assert_eq!(
        rec1_json["items"][0]["quantity_received"]
            .as_str()
            .unwrap()
            .parse::<Decimal>()
            .unwrap(),
        Decimal::from(20)
    );

    // Stock balance should now be 20
    let bal2_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/inventory/balance/{}", product_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let bal2_res = app.clone().oneshot(bal2_req).await.unwrap();
    let bal2_bytes = axum::body::to_bytes(bal2_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let bal2_json: Value = serde_json::from_slice(&bal2_bytes).unwrap();
    assert_eq!(
        bal2_json["on_hand_quantity"]
            .as_str()
            .unwrap()
            .parse::<Decimal>()
            .unwrap(),
        Decimal::from(20)
    );

    // 6. Receive remaining 30 units (full fulfillment)
    let rec2_req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/v1/procurement/purchase-orders/{}/receive",
            po_id
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "items": [
                    {
                        "item_id": po_item_id,
                        "quantity_received": "30.00",
                        "batch_number": "BATCH-2026-A2"
                    }
                ]
            })
            .to_string(),
        ))
        .unwrap();

    let rec2_res = app.clone().oneshot(rec2_req).await.unwrap();
    assert_eq!(rec2_res.status(), StatusCode::OK);
    let rec2_bytes = axum::body::to_bytes(rec2_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let rec2_json: Value = serde_json::from_slice(&rec2_bytes).unwrap();
    assert_eq!(rec2_json["status"], "RECEIVED");
    assert_eq!(
        rec2_json["items"][0]["quantity_received"]
            .as_str()
            .unwrap()
            .parse::<Decimal>()
            .unwrap(),
        Decimal::from(50)
    );

    // Stock balance should now be exactly 50
    let bal3_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/inventory/balance/{}", product_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let bal3_res = app.clone().oneshot(bal3_req).await.unwrap();
    let bal3_bytes = axum::body::to_bytes(bal3_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let bal3_json: Value = serde_json::from_slice(&bal3_bytes).unwrap();
    assert_eq!(
        bal3_json["on_hand_quantity"]
            .as_str()
            .unwrap()
            .parse::<Decimal>()
            .unwrap(),
        Decimal::from(50)
    );

    // 7. Test PO Cancellation on a fresh PO
    let po2_req = Request::builder()
        .method("POST")
        .uri("/api/v1/procurement/purchase-orders")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "supplier_id": supplier_id,
                "items": [
                    {
                        "product_id": product_id,
                        "item_name": "Industrial Router X500",
                        "quantity_ordered": "10.00",
                        "unit_cost": "120.00"
                    }
                ]
            })
            .to_string(),
        ))
        .unwrap();

    let po2_res = app.clone().oneshot(po2_req).await.unwrap();
    assert_eq!(po2_res.status(), StatusCode::CREATED);
    let po2_bytes = axum::body::to_bytes(po2_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let po2_json: Value = serde_json::from_slice(&po2_bytes).unwrap();
    let po2_id = po2_json["id"].as_str().unwrap();

    let cancel_req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/v1/procurement/purchase-orders/{}/cancel",
            po2_id
        ))
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let cancel_res = app.clone().oneshot(cancel_req).await.unwrap();
    assert_eq!(cancel_res.status(), StatusCode::OK);
    let cancel_bytes = axum::body::to_bytes(cancel_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let cancel_json: Value = serde_json::from_slice(&cancel_bytes).unwrap();
    assert_eq!(cancel_json["status"], "CANCELLED");
}

#[tokio::test]
async fn test_promotions_and_coupons_engine() {
    let db = establish_connection("sqlite::memory:")
        .await
        .expect("in-memory db");
    run_migrations(&db).await.expect("migrations");

    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let jwt_secret = "secret_promo_test_123456";
    let token = generate_token(user_id, TenantId(tenant_id), "admin", jwt_secret, 24).unwrap();

    let state = Arc::new(AppState::new(db.clone(), jwt_secret));
    let app = create_router(state);

    // 1. Create a Percentage Coupon with Threshold & Cap
    let cp1_req = Request::builder()
        .method("POST")
        .uri("/api/v1/promotions/coupons")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "code": "SUMMER20",
                "discount_type": "PERCENTAGE",
                "discount_value": "0.20",
                "min_order_amount": "50.00",
                "max_discount_amount": "30.00",
                "usage_limit": 2
            })
            .to_string(),
        ))
        .unwrap();

    let cp1_res = app.clone().oneshot(cp1_req).await.unwrap();
    assert_eq!(cp1_res.status(), StatusCode::CREATED);

    // 2. Validate below min order threshold ($40 < $50)
    let val1_req = Request::builder()
        .method("POST")
        .uri("/api/v1/promotions/coupons/validate")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "code": "summer20",
                "subtotal": "40.00"
            })
            .to_string(),
        ))
        .unwrap();

    let val1_res = app.clone().oneshot(val1_req).await.unwrap();
    assert_eq!(val1_res.status(), StatusCode::OK);
    let val1_bytes = axum::body::to_bytes(val1_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let val1_json: Value = serde_json::from_slice(&val1_bytes).unwrap();
    assert_eq!(val1_json["is_valid"], false);
    assert_eq!(
        val1_json["discount_amount"]
            .as_str()
            .unwrap()
            .parse::<Decimal>()
            .unwrap(),
        Decimal::ZERO
    );

    // 3. Validate above threshold ($100 * 20% = $20 discount)
    let val2_req = Request::builder()
        .method("POST")
        .uri("/api/v1/promotions/coupons/validate")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "code": "SUMMER20",
                "subtotal": "100.00"
            })
            .to_string(),
        ))
        .unwrap();

    let val2_res = app.clone().oneshot(val2_req).await.unwrap();
    assert_eq!(val2_res.status(), StatusCode::OK);
    let val2_bytes = axum::body::to_bytes(val2_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let val2_json: Value = serde_json::from_slice(&val2_bytes).unwrap();
    assert_eq!(val2_json["is_valid"], true);
    assert_eq!(
        val2_json["discount_amount"]
            .as_str()
            .unwrap()
            .parse::<Decimal>()
            .unwrap(),
        Decimal::from(20)
    );
    assert_eq!(
        val2_json["final_subtotal"]
            .as_str()
            .unwrap()
            .parse::<Decimal>()
            .unwrap(),
        Decimal::from(80)
    );

    // 4. Validate above max discount cap ($250 * 20% = $50, capped to $30)
    let val3_req = Request::builder()
        .method("POST")
        .uri("/api/v1/promotions/coupons/validate")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "code": "SUMMER20",
                "subtotal": "250.00"
            })
            .to_string(),
        ))
        .unwrap();

    let val3_res = app.clone().oneshot(val3_req).await.unwrap();
    assert_eq!(val3_res.status(), StatusCode::OK);
    let val3_bytes = axum::body::to_bytes(val3_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let val3_json: Value = serde_json::from_slice(&val3_bytes).unwrap();
    assert_eq!(val3_json["is_valid"], true);
    assert_eq!(
        val3_json["discount_amount"]
            .as_str()
            .unwrap()
            .parse::<Decimal>()
            .unwrap(),
        Decimal::from(30)
    );
    assert_eq!(
        val3_json["final_subtotal"]
            .as_str()
            .unwrap()
            .parse::<Decimal>()
            .unwrap(),
        Decimal::from(220)
    );

    // 5. Redeem coupon (1st usage)
    let red1_req = Request::builder()
        .method("POST")
        .uri("/api/v1/promotions/coupons/SUMMER20/redeem")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "subtotal": "100.00"
            })
            .to_string(),
        ))
        .unwrap();

    let red1_res = app.clone().oneshot(red1_req).await.unwrap();
    assert_eq!(red1_res.status(), StatusCode::OK);
    let red1_bytes = axum::body::to_bytes(red1_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let red1_json: Value = serde_json::from_slice(&red1_bytes).unwrap();
    assert_eq!(red1_json["used_count"], 1);
    assert_eq!(red1_json["remaining_usage"], 1);

    // 6. Redeem coupon (2nd usage - reaches limit of 2)
    let red2_req = Request::builder()
        .method("POST")
        .uri("/api/v1/promotions/coupons/SUMMER20/redeem")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "subtotal": "100.00"
            })
            .to_string(),
        ))
        .unwrap();

    let red2_res = app.clone().oneshot(red2_req).await.unwrap();
    assert_eq!(red2_res.status(), StatusCode::OK);
    let red2_bytes = axum::body::to_bytes(red2_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let red2_json: Value = serde_json::from_slice(&red2_bytes).unwrap();
    assert_eq!(red2_json["used_count"], 2);
    assert_eq!(red2_json["remaining_usage"], 0);

    // 7. 3rd redemption must fail with BAD_REQUEST
    let red3_req = Request::builder()
        .method("POST")
        .uri("/api/v1/promotions/coupons/SUMMER20/redeem")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "subtotal": "100.00"
            })
            .to_string(),
        ))
        .unwrap();

    let red3_res = app.clone().oneshot(red3_req).await.unwrap();
    assert_eq!(red3_res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_multi_currency_exchange_rates_and_conversion() {
    let db = establish_connection("sqlite::memory:")
        .await
        .expect("in-memory db");
    run_migrations(&db).await.expect("migrations");

    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let jwt_secret = "secret_currency_test_123456";
    let token = generate_token(user_id, TenantId(tenant_id), "admin", jwt_secret, 24).unwrap();

    let state = Arc::new(AppState::new(db.clone(), jwt_secret));
    let app = create_router(state);

    // 1. Set Exchange Rate USD -> INR = 84.50
    let set_rate_req = Request::builder()
        .method("POST")
        .uri("/api/v1/billing/exchange-rates")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "from_currency": "USD",
                "to_currency": "INR",
                "rate": "84.50"
            })
            .to_string(),
        ))
        .unwrap();

    let set_res = app.clone().oneshot(set_rate_req).await.unwrap();
    assert_eq!(set_res.status(), StatusCode::OK);
    let set_bytes = axum::body::to_bytes(set_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let set_json: Value = serde_json::from_slice(&set_bytes).unwrap();
    assert_eq!(set_json["from_currency"], "USD");
    assert_eq!(set_json["to_currency"], "INR");
    assert_eq!(
        set_json["rate"]
            .as_str()
            .unwrap()
            .parse::<Decimal>()
            .unwrap(),
        Decimal::from_str("84.50").unwrap()
    );

    // 2. List exchange rates
    let list_req = Request::builder()
        .method("GET")
        .uri("/api/v1/billing/exchange-rates")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let list_res = app.clone().oneshot(list_req).await.unwrap();
    assert_eq!(list_res.status(), StatusCode::OK);

    // 3. Convert 100 USD -> INR (Direct lookup)
    let conv1_req = Request::builder()
        .method("POST")
        .uri("/api/v1/billing/convert-currency")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "amount": "100.00",
                "from_currency": "USD",
                "to_currency": "INR"
            })
            .to_string(),
        ))
        .unwrap();

    let conv1_res = app.clone().oneshot(conv1_req).await.unwrap();
    assert_eq!(conv1_res.status(), StatusCode::OK);
    let conv1_bytes = axum::body::to_bytes(conv1_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let conv1_json: Value = serde_json::from_slice(&conv1_bytes).unwrap();
    assert_eq!(
        conv1_json["converted_amount"]
            .as_str()
            .unwrap()
            .parse::<Decimal>()
            .unwrap(),
        Decimal::from_str("8450.00").unwrap()
    );

    // 4. Convert 8450 INR -> USD (Inverse lookup)
    let conv2_req = Request::builder()
        .method("POST")
        .uri("/api/v1/billing/convert-currency")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "amount": "8450.00",
                "from_currency": "INR",
                "to_currency": "USD"
            })
            .to_string(),
        ))
        .unwrap();

    let conv2_res = app.clone().oneshot(conv2_req).await.unwrap();
    assert_eq!(conv2_res.status(), StatusCode::OK);
    let conv2_bytes = axum::body::to_bytes(conv2_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let conv2_json: Value = serde_json::from_slice(&conv2_bytes).unwrap();
    assert_eq!(
        conv2_json["converted_amount"]
            .as_str()
            .unwrap()
            .parse::<Decimal>()
            .unwrap(),
        Decimal::from_str("100.00").unwrap()
    );
}
