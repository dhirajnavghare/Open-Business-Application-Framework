use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use obaf_api::create_router;
use obaf_api::state::AppState;
use obaf_database::{migration::run_migrations, pool::establish_connection};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_security_headers_middleware() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    run_migrations(&db).await.unwrap();

    let state = Arc::new(AppState::new(db, "test_secret_32_characters_minimum_len"));
    let app = create_router(state);

    let req = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let headers = res.headers();
    assert!(headers.get("x-request-id").is_some());
    assert_eq!(
        headers
            .get("x-content-type-options")
            .unwrap()
            .to_str()
            .unwrap(),
        "nosniff"
    );
    assert_eq!(
        headers.get("x-frame-options").unwrap().to_str().unwrap(),
        "DENY"
    );
    assert_eq!(
        headers.get("x-xss-protection").unwrap().to_str().unwrap(),
        "1; mode=block"
    );
    assert!(headers
        .get("strict-transport-security")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("max-age=31536000"));
    assert!(headers
        .get("content-security-policy")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("default-src 'self'"));
}

#[tokio::test]
async fn test_token_bucket_rate_limiter_middleware() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    run_migrations(&db).await.unwrap();

    let state = Arc::new(AppState::new(db, "test_secret_32_characters_minimum_len"));
    let app = create_router(state.clone());

    // 1. Health check should bypass rate limiting
    let health_req = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let health_res = app.clone().oneshot(health_req).await.unwrap();
    assert_eq!(health_res.status(), StatusCode::OK);

    // 2. An API request with a client key should include x-ratelimit-remaining header
    let api_req = Request::builder()
        .method("GET")
        .uri("/api/v1/customers")
        .header("x-forwarded-for", "203.0.113.195")
        .header("Authorization", "Bearer dummy-token")
        .body(Body::empty())
        .unwrap();
    let api_res = app.clone().oneshot(api_req).await.unwrap();
    assert!(api_res.headers().get("x-ratelimit-remaining").is_some());

    // 3. Exhaust tokens for a specific client key directly via state.rate_limiter
    let client_key = "ip:198.51.100.42";
    state.rate_limiter.check_and_consume(client_key, 100.0);

    // Next request from that client IP should trigger HTTP 429 Too Many Requests
    let limited_req = Request::builder()
        .method("GET")
        .uri("/api/v1/customers")
        .header("x-forwarded-for", "198.51.100.42")
        .body(Body::empty())
        .unwrap();
    let limited_res = app.oneshot(limited_req).await.unwrap();
    assert_eq!(limited_res.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(limited_res.headers().get("retry-after").is_some());
    assert_eq!(
        limited_res
            .headers()
            .get("x-ratelimit-remaining")
            .unwrap()
            .to_str()
            .unwrap(),
        "0"
    );
}

#[tokio::test]
async fn test_saas_tenant_provisioning_and_usage() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    run_migrations(&db).await.unwrap();

    let state = Arc::new(AppState::new(db, "test_secret_32_characters_minimum_len"));
    let app = create_router(state);

    // 1. Provision SaaS tenant
    let prov_req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/tenants")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "organization_name": "Apex Fitness Chain",
                "slug": "apex-fitness",
                "preset_id": "gym_fitness",
                "plan_tier": "starter",
                "admin_email": "owner@apexfitness.test",
                "admin_name": "Coach Dave",
                "admin_password": "DaveSecurePassword123!"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(prov_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let prov_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let tenant_id = prov_json["tenant_id"].as_str().unwrap();
    let token = prov_json["access_token"].as_str().unwrap();

    // 2. Query usage metering
    let usage_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/admin/tenants/{}/usage", tenant_id))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(usage_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let usage_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(usage_json["plan_tier"], "starter");
    assert_eq!(usage_json["products_limit"], 100);
    assert_eq!(usage_json["orders_limit"], 500);
    assert_eq!(usage_json["is_within_quota"], true);

    // 3. Test dynamic rules engine
    let rules_req = Request::builder()
        .method("POST")
        .uri("/api/v1/billing/apply-rules")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "subtotal": "200.00",
                "promo_code": "FITNESS20",
                "rules": [
                    {
                        "id": "promo_code_20",
                        "name": "20% off with coupon",
                        "condition": { "type": "PromoCode", "value": "FITNESS20" },
                        "action": { "type": "PercentageDiscount", "value": "0.20" }
                    }
                ]
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(rules_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let rules_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(rules_json["adjusted_subtotal"], "160.00");

    // 4. Test offline sync push & idempotency
    let action_id = Uuid::new_v4();
    let sync_push_req = Request::builder()
        .method("POST")
        .uri("/api/v1/sync/push")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "actions": [
                    {
                        "client_action_id": action_id,
                        "action_type": "create_order",
                        "payload": {
                            "order_number": "OFFLINE-ORD-101",
                            "subtotal": "50.00",
                            "grand_total": "50.00",
                            "customer_name": "Walk-in Gymnast"
                        },
                        "client_timestamp": Utc::now()
                    }
                ]
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(sync_push_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let push_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(push_json["processed"], 1);
    assert_eq!(push_json["results"][0]["status"], "applied");

    // Idempotent retry: Push same action again
    let sync_retry_req = Request::builder()
        .method("POST")
        .uri("/api/v1/sync/push")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "actions": [
                    {
                        "client_action_id": action_id,
                        "action_type": "create_order",
                        "payload": {
                            "order_number": "OFFLINE-ORD-101",
                            "subtotal": "50.00"
                        },
                        "client_timestamp": Utc::now()
                    }
                ]
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(sync_retry_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let retry_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(retry_json["results"][0]["status"], "duplicate_ignored");

    // 5. Test offline sync pull
    let sync_pull_req = Request::builder()
        .method("GET")
        .uri("/api/v1/sync/pull")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(sync_pull_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let pull_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(pull_json["orders_count"], 1);
}
