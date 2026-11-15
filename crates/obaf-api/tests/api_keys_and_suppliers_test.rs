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
async fn test_api_keys_and_suppliers_lifecycle() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    run_migrations(&db).await.unwrap();

    let jwt_secret = "test_secret_32_characters_minimum_len";
    let state = Arc::new(AppState::new(db, jwt_secret));
    let app = create_router(state);

    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let bearer_token =
        generate_token(user_id, TenantId(tenant_id), "admin", jwt_secret, 24).unwrap();

    // 1. Create API Key
    let create_key_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/api-keys")
        .header("Authorization", format!("Bearer {}", bearer_token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "name": "POS Gateway Terminal #01",
                "scopes": ["*"],
                "expires_in_days": 30
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(create_key_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let key_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let raw_api_key = key_json["key"].as_str().unwrap().to_string();
    let key_id = key_json["id"].as_str().unwrap().to_string();

    // 2. Query suppliers using X-API-Key (Machine-to-Machine Auth)
    let list_req = Request::builder()
        .method("GET")
        .uri("/api/v1/suppliers")
        .header("X-API-Key", &raw_api_key)
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(list_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. Create a Supplier using X-API-Key
    let create_supplier_req = Request::builder()
        .method("POST")
        .uri("/api/v1/suppliers")
        .header("X-API-Key", &raw_api_key)
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "name": "Global Farm Supplies Ltd",
                "contact_person": "Robert Green",
                "phone": "+1-555-8899",
                "email": "procurement@globalfarm.test",
                "tax_id": "GSTIN998877",
                "address": "45 Industrial Zone",
                "metadata": {
                    "lead_time_days": 3,
                    "payment_terms": "NET30"
                }
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(create_supplier_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let supp_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let supplier_id = supp_json["id"].as_str().unwrap();

    // 4. Retrieve Supplier by ID
    let get_supp_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/suppliers/{}", supplier_id))
        .header("X-API-Key", &raw_api_key)
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(get_supp_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 5. Revoke API Key
    let revoke_req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/auth/api-keys/{}", key_id))
        .header("Authorization", format!("Bearer {}", bearer_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(revoke_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 6. Verify revoked API key is rejected
    let rejected_req = Request::builder()
        .method("GET")
        .uri("/api/v1/suppliers")
        .header("X-API-Key", &raw_api_key)
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(rejected_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
