use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use obaf_api::{create_router, AppState};
use obaf_core::{generate_token, TenantId};
use obaf_database::{establish_connection, run_migrations};
use rust_decimal::Decimal;
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_pos_shifts_credit_notes_and_locations() {
    let db = establish_connection("sqlite::memory:")
        .await
        .expect("in-memory db");
    run_migrations(&db).await.expect("migrations");

    let tenant_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();
    let cashier_id = Uuid::new_v4();
    let jwt_secret = "secret_jwt_pos_shifts_123456";
    let admin_token =
        generate_token(admin_id, TenantId(tenant_id), "admin", jwt_secret, 24).unwrap();
    let cashier_token =
        generate_token(cashier_id, TenantId(tenant_id), "cashier", jwt_secret, 24).unwrap();

    let state = Arc::new(AppState::new(db.clone(), jwt_secret));
    let app = create_router(state);

    // 1. Create a stock location (requires admin/manager)
    let loc_req = Request::builder()
        .method("POST")
        .uri("/api/v1/inventory/locations")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "name": "Central Retail Stockroom",
                "code": "WH-CENTRAL",
                "address": "Terminal 2, Bay 4"
            })
            .to_string(),
        ))
        .unwrap();

    let loc_res = app.clone().oneshot(loc_req).await.unwrap();
    assert_eq!(loc_res.status(), StatusCode::CREATED);
    let loc_bytes = axum::body::to_bytes(loc_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let loc_json: Value = serde_json::from_slice(&loc_bytes).unwrap();
    assert_eq!(loc_json["code"], "WH-CENTRAL");

    // List stock locations
    let list_loc_req = Request::builder()
        .method("GET")
        .uri("/api/v1/inventory/locations")
        .header(header::AUTHORIZATION, format!("Bearer {}", cashier_token))
        .body(Body::empty())
        .unwrap();

    let list_loc_res = app.clone().oneshot(list_loc_req).await.unwrap();
    assert_eq!(list_loc_res.status(), StatusCode::OK);

    // 2. Open a POS cashier shift
    let open_shift_req = Request::builder()
        .method("POST")
        .uri("/api/v1/pos/shifts/open")
        .header(header::AUTHORIZATION, format!("Bearer {}", cashier_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "register_name": "Counter 1 - Fast Lane",
                "opening_float": "150.00"
            })
            .to_string(),
        ))
        .unwrap();

    let open_res = app.clone().oneshot(open_shift_req).await.unwrap();
    assert_eq!(open_res.status(), StatusCode::CREATED);
    let open_bytes = axum::body::to_bytes(open_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let open_json: Value = serde_json::from_slice(&open_bytes).unwrap();
    let shift_id = open_json["id"].as_str().unwrap();
    assert_eq!(open_json["status"], "OPEN");

    // 3. Query current active shift
    let cur_req = Request::builder()
        .method("GET")
        .uri("/api/v1/pos/shifts/current")
        .header(header::AUTHORIZATION, format!("Bearer {}", cashier_token))
        .body(Body::empty())
        .unwrap();

    let cur_res = app.clone().oneshot(cur_req).await.unwrap();
    assert_eq!(cur_res.status(), StatusCode::OK);
    let cur_bytes = axum::body::to_bytes(cur_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let cur_json: Value = serde_json::from_slice(&cur_bytes).unwrap();
    assert_eq!(cur_json["id"].as_str().unwrap(), shift_id);

    // 4. Close the shift with counted cash ($155.00 => $5.00 surplus variance)
    let close_req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/pos/shifts/{}/close", shift_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", cashier_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "counted_cash": "155.00",
                "notes": "Evening shift drawer balanced with small cash tip surplus"
            })
            .to_string(),
        ))
        .unwrap();

    let close_res = app.clone().oneshot(close_req).await.unwrap();
    assert_eq!(close_res.status(), StatusCode::OK);
    let close_bytes = axum::body::to_bytes(close_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let close_json: Value = serde_json::from_slice(&close_bytes).unwrap();
    assert_eq!(close_json["status"], "CLOSED");

    let variance_str = close_json["variance"].as_str().unwrap();
    let variance: Decimal = variance_str.parse().unwrap();
    assert_eq!(variance, Decimal::from(5));

    // 5. Issue an immutable Credit Note (requires admin/manager)
    let cn_req = Request::builder()
        .method("POST")
        .uri("/api/v1/billing/credit-notes")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "amount": "45.00",
                "tax_amount": "2.25",
                "reason": "Customer returned unopened merchandise"
            })
            .to_string(),
        ))
        .unwrap();

    let cn_res = app.clone().oneshot(cn_req).await.unwrap();
    assert_eq!(cn_res.status(), StatusCode::CREATED);
    let cn_bytes = axum::body::to_bytes(cn_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let cn_json: Value = serde_json::from_slice(&cn_bytes).unwrap();
    let cn_id = cn_json["id"].as_str().unwrap();
    assert_eq!(cn_json["status"], "ISSUED");

    // Query credit note details
    let get_cn_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/billing/credit-notes/{}", cn_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", cashier_token))
        .body(Body::empty())
        .unwrap();

    let get_cn_res = app.oneshot(get_cn_req).await.unwrap();
    assert_eq!(get_cn_res.status(), StatusCode::OK);
}
