use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use obaf_api::router::create_router;
use obaf_api::state::AppState;
use obaf_core::security::generate_token;
use obaf_core::tenant::TenantId;
use obaf_database::migration::run_migrations;
use obaf_database::pool::establish_connection;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_tenant_data_isolation() {
    // 1. Establish in-memory database and run migrations
    let db = establish_connection("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    run_migrations(&db).await.expect("Failed to run migrations");

    let jwt_secret = "test-secret-key-very-secure-12345";
    let state = Arc::new(AppState::new(db, jwt_secret));
    let app = create_router(state);

    // 2. Setup Tenant A and Tenant B
    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();

    let token_a = generate_token(user_a, TenantId::from(tenant_a), "admin", jwt_secret, 1)
        .expect("Failed to generate token A");
    let token_b = generate_token(user_b, TenantId::from(tenant_b), "admin", jwt_secret, 1)
        .expect("Failed to generate token B");

    // 3. Tenant A creates a customer
    let create_customer_req = Request::builder()
        .method("POST")
        .uri("/api/v1/customers")
        .header("Authorization", format!("Bearer {}", token_a))
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "name": "Acme Corp (Tenant A)",
                "email": "contact@acme.corp",
                "phone": "+1-555-0199"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(create_customer_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let customer_a: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let customer_a_id = customer_a["id"].as_str().unwrap();

    // 4. Tenant B lists customers -> Must NOT see Tenant A's customer
    let list_req_b = Request::builder()
        .method("GET")
        .uri("/api/v1/customers")
        .header("Authorization", format!("Bearer {}", token_b))
        .body(Body::empty())
        .unwrap();

    let res_b = app.clone().oneshot(list_req_b).await.unwrap();
    assert_eq!(res_b.status(), StatusCode::OK);
    let body_bytes_b = axum::body::to_bytes(res_b.into_body(), usize::MAX)
        .await
        .unwrap();
    let customers_b: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes_b).unwrap();
    assert!(
        customers_b.is_empty(),
        "Tenant B should not see Tenant A's customers"
    );

    // 5. Tenant B attempts direct access to Tenant A's customer -> Must return 404 Not Found
    let get_req_b = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/customers/{}", customer_a_id))
        .header("Authorization", format!("Bearer {}", token_b))
        .body(Body::empty())
        .unwrap();

    let res_direct = app.clone().oneshot(get_req_b).await.unwrap();
    assert_eq!(
        res_direct.status(),
        StatusCode::NOT_FOUND,
        "Tenant B cannot access Tenant A's customer by ID"
    );

    // 6. Unauthenticated request -> Must return 401 Unauthorized
    let unauth_req = Request::builder()
        .method("GET")
        .uri("/api/v1/customers")
        .body(Body::empty())
        .unwrap();

    let res_unauth = app.clone().oneshot(unauth_req).await.unwrap();
    assert_eq!(
        res_unauth.status(),
        StatusCode::UNAUTHORIZED,
        "Unauthenticated request must be rejected"
    );
}
