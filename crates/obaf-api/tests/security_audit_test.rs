// Copyright 2026 Dhiraj Navghare
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use obaf_api::create_router;
use obaf_api::state::AppState;
use obaf_core::security::generate_token;
use obaf_core::tenant::TenantId;
use obaf_database::{entities::product, migration::run_migrations, pool::establish_connection};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

const TEST_SECRET: &str = "test-secret-at-least-32-chars-long-security-suite!";

async fn setup_test_app() -> (axum::Router, Arc<AppState>) {
    let db = establish_connection("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory db");
    run_migrations(&db).await.expect("Failed to run migrations");
    let state = Arc::new(AppState::new(db, TEST_SECRET));
    let app = create_router(state.clone());
    (app, state)
}

#[tokio::test]
async fn test_security_headers_and_csp() {
    let (app, _) = setup_test_app().await;

    let req = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let headers = res.headers();
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

    let csp = headers
        .get("content-security-policy")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(csp.contains("default-src 'self'"));
    assert!(csp.contains("script-src 'self'"));
    assert!(csp.contains("object-src 'none'"));
    assert!(csp.contains("base-uri 'self'"));

    assert_eq!(
        headers
            .get("cross-origin-resource-policy")
            .unwrap()
            .to_str()
            .unwrap(),
        "same-origin"
    );
    assert_eq!(
        headers
            .get("cross-origin-opener-policy")
            .unwrap()
            .to_str()
            .unwrap(),
        "same-origin-allow-popups"
    );
    assert!(headers
        .get("permissions-policy")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("geolocation=()"));
}

#[tokio::test]
async fn test_path_traversal_protection_in_setup() {
    let (app, _) = setup_test_app().await;

    // 1. Directory traversal attempt with ../
    let traversal_payload = json!({
        "slug": "acme-corp",
        "organization_name": "Acme Corp",
        "preset": "../../etc/passwd",
        "admin_name": "Admin",
        "admin_email": "admin@acme.com",
        "admin_password": "Password123!"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/setup/preset")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(traversal_payload.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body["error"].as_str().unwrap().contains("Invalid preset"));

    // 2. Command injection / shell character attempt in preset
    let injection_payload = json!({
        "slug": "acme-corp",
        "organization_name": "Acme Corp",
        "preset": "retail;rm -rf /",
        "admin_name": "Admin",
        "admin_email": "admin@acme.com",
        "admin_password": "Password123!"
    });

    let req2 = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/setup/preset")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(injection_payload.to_string()))
        .unwrap();

    let res2 = app.clone().oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::BAD_REQUEST);

    // 3. Invalid characters in slug
    let bad_slug_payload = json!({
        "slug": "acme/corp/..",
        "organization_name": "Acme Corp",
        "preset": "retail",
        "admin_name": "Admin",
        "admin_email": "admin@acme.com",
        "admin_password": "Password123!"
    });

    let req3 = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/setup/preset")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(bad_slug_payload.to_string()))
        .unwrap();

    let res3 = app.oneshot(req3).await.unwrap();
    assert_eq!(res3.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_ssrf_protection_in_webhooks() {
    let (app, _) = setup_test_app().await;

    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let token =
        generate_token(user_id, TenantId::from(tenant_id), "admin", TEST_SECRET, 1).unwrap();

    // 1. AWS/Cloud instance metadata IP SSRF attempt
    let metadata_ip_payload = json!({
        "url": "http://169.254.169.254/latest/meta-data/",
        "event_types": ["order.created"]
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/webhooks")
        .header("Authorization", format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(metadata_ip_payload.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body["error"].as_str().unwrap().contains("metadata"));

    // 2. GCP metadata domain SSRF attempt
    let gcp_metadata_payload = json!({
        "url": "http://metadata.google.internal/computeMetadata/v1/",
        "event_types": ["order.created"]
    });

    let req2 = Request::builder()
        .method("POST")
        .uri("/api/v1/webhooks")
        .header("Authorization", format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(gcp_metadata_payload.to_string()))
        .unwrap();

    let res2 = app.clone().oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::BAD_REQUEST);

    // 3. Localhost loopback SSRF attempt
    let loopback_payload = json!({
        "url": "http://127.0.0.1:8080/internal-api",
        "event_types": ["order.created"]
    });

    let req3 = Request::builder()
        .method("POST")
        .uri("/api/v1/webhooks")
        .header("Authorization", format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(loopback_payload.to_string()))
        .unwrap();

    let res3 = app.clone().oneshot(req3).await.unwrap();
    assert_eq!(res3.status(), StatusCode::BAD_REQUEST);

    // 4. File protocol attempt
    let file_proto_payload = json!({
        "url": "file:///etc/passwd",
        "event_types": ["order.created"]
    });

    let req4 = Request::builder()
        .method("POST")
        .uri("/api/v1/webhooks")
        .header("Authorization", format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(file_proto_payload.to_string()))
        .unwrap();

    let res4 = app.clone().oneshot(req4).await.unwrap();
    assert_eq!(res4.status(), StatusCode::BAD_REQUEST);

    // 5. Valid HTTPS webhook registration succeeds
    let valid_payload = json!({
        "url": "https://webhook.example.com/events",
        "event_types": ["order.created"]
    });

    let req5 = Request::builder()
        .method("POST")
        .uri("/api/v1/webhooks")
        .header("Authorization", format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(valid_payload.to_string()))
        .unwrap();

    let res5 = app.oneshot(req5).await.unwrap();
    assert_eq!(res5.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_unauthenticated_websocket_rejection() {
    let (app, _) = setup_test_app().await;

    // 1. Connection without any auth header or query param -> 401 Unauthorized
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ws/events")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 2. Connection with invalid JWT token in query param -> 401 Unauthorized
    let req2 = Request::builder()
        .method("GET")
        .uri("/api/v1/ws/events?token=invalid.jwt.token")
        .body(Body::empty())
        .unwrap();

    let res2 = app.oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_request_body_size_limit_dos_protection() {
    let (app, _) = setup_test_app().await;

    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let token =
        generate_token(user_id, TenantId::from(tenant_id), "admin", TEST_SECRET, 1).unwrap();

    // Create a payload larger than 2MB (2 * 1024 * 1024 = 2,097,152 bytes)
    // 2.5 MB of dummy data
    let oversized_data = vec![b'A'; 2_500_000];

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/customers")
        .header("Authorization", format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(oversized_data))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn test_tenant_rate_limiting_bucket_isolation() {
    let (app, state) = setup_test_app().await;

    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();

    let token_a =
        generate_token(user_a, TenantId::from(tenant_a), "admin", TEST_SECRET, 1).unwrap();
    let token_b =
        generate_token(user_b, TenantId::from(tenant_b), "admin", TEST_SECRET, 1).unwrap();

    // 1. Send request with Tenant A
    let req_a = Request::builder()
        .method("GET")
        .uri("/api/v1/customers")
        .header("Authorization", format!("Bearer {}", token_a))
        .body(Body::empty())
        .unwrap();

    let res_a = app.clone().oneshot(req_a).await.unwrap();
    assert_eq!(res_a.status(), StatusCode::OK);

    // Verify rate limit remaining header is present
    let remaining_a = res_a
        .headers()
        .get("x-ratelimit-remaining")
        .expect("missing x-ratelimit-remaining header")
        .to_str()
        .unwrap();
    assert_eq!(remaining_a, "99");

    // 2. Consume remaining 99 tokens for Tenant A's isolated bucket
    let key_a = format!("tenant:{}", tenant_a);
    state.rate_limiter.check_and_consume(&key_a, 99.0);

    // 3. Tenant A is now rate limited (HTTP 429)
    let req_a_blocked = Request::builder()
        .method("GET")
        .uri("/api/v1/customers")
        .header("Authorization", format!("Bearer {}", token_a))
        .body(Body::empty())
        .unwrap();

    let res_a_blocked = app.clone().oneshot(req_a_blocked).await.unwrap();
    assert_eq!(res_a_blocked.status(), StatusCode::TOO_MANY_REQUESTS);

    // 4. Tenant B is NOT rate limited and has full quota available (tenant isolation)
    let req_b = Request::builder()
        .method("GET")
        .uri("/api/v1/customers")
        .header("Authorization", format!("Bearer {}", token_b))
        .body(Body::empty())
        .unwrap();

    let res_b = app.oneshot(req_b).await.unwrap();
    assert_eq!(res_b.status(), StatusCode::OK);
    let remaining_b = res_b
        .headers()
        .get("x-ratelimit-remaining")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(remaining_b, "99");
}

#[tokio::test]
async fn test_stored_xss_protection_in_invoice_html() {
    let (app, state) = setup_test_app().await;

    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let token =
        generate_token(user_id, TenantId::from(tenant_id), "admin", TEST_SECRET, 1).unwrap();

    // 1. Insert a product directly into the database with XSS payload in its name
    let product_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let p = product::ActiveModel {
        id: Set(product_id),
        tenant_id: Set(tenant_id),
        sku: Set("XSS-PROD-01".to_string()),
        name: Set("<script>alert('XSS-Item')</script>".to_string()),
        category_id: Set(None),
        price: Set(Decimal::from(25)),
        cost_price: Set(Decimal::from(10)),
        tax_rate: Set(Decimal::new(1, 1)),
        is_active: Set(true),
        metadata: Set("{}".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };
    p.insert(&state.db).await.unwrap();

    // 2. Create an order with XSS in table_or_room and customer_name
    let order_payload = json!({
        "customer_name": "<script>alert('XSS-Customer')</script>",
        "table_or_room": "Table <img src=x onerror=alert(1)>",
        "items": [
            {
                "product_id": product_id,
                "quantity": 2
            }
        ]
    });

    let req2 = Request::builder()
        .method("POST")
        .uri("/api/v1/orders")
        .header("Authorization", format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(order_payload.to_string()))
        .unwrap();

    let res2 = app.clone().oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::CREATED);
    let bytes2 = to_bytes(res2.into_body(), usize::MAX).await.unwrap();
    let ord: serde_json::Value = serde_json::from_slice(&bytes2).unwrap();
    let order_id = ord["id"].as_str().unwrap();

    // 3. Request the rendered HTML invoice
    let req_html = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/billing/orders/{}/invoice.html", order_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res_html = app.oneshot(req_html).await.unwrap();
    assert_eq!(res_html.status(), StatusCode::OK);
    let html_bytes = to_bytes(res_html.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8(html_bytes.to_vec()).unwrap();

    // 4. Verify raw executable script tags do NOT appear unescaped
    assert!(!html.contains("<script>alert('XSS-Item')</script>"));
    assert!(!html.contains("<script>alert('XSS-Customer')</script>"));
    assert!(!html.contains("<img src=x onerror=alert(1)>"));

    // 5. Verify proper HTML entities are escaped
    assert!(html.contains("&lt;script&gt;alert(&#39;XSS-Item&#39;)&lt;/script&gt;"));
    assert!(html.contains("&lt;script&gt;alert(&#39;XSS-Customer&#39;)&lt;/script&gt;"));
    assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
}

#[tokio::test]
async fn test_domain_validation_and_injection_defense() {
    let (app, _) = setup_test_app().await;

    // 1. Authenticate as Super Admin
    let login_body = json!({
        "email": "admin@obaf.local",
        "password": "admin123"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(login_body.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let login_res: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let admin_token = login_res["token"].as_str().unwrap();

    // 2. Reject domain containing newlines (Caddy/Nginx CRLF config injection attempt)
    let crlf_domain = json!({
        "domain": "api.example.com\nserver {\nlisten 80;\n}",
        "email": "admin@example.com"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/ssl/configure-domain")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(crlf_domain.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 3. Reject raw loopback / private IP as domain
    let ip_domain = json!({
        "domain": "127.0.0.1",
        "email": "admin@example.com"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/ssl/configure-domain")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(ip_domain.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 4. Reject localhost as domain
    let localhost_domain = json!({
        "domain": "localhost",
        "email": "admin@example.com"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/ssl/configure-domain")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(localhost_domain.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 5. Reject domain with command injection characters
    let cmd_domain = json!({
        "domain": "api.example.com; rm -rf /",
        "email": "admin@example.com"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/ssl/configure-domain")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(cmd_domain.to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_acme_challenge_token_security() {
    let (app, _) = setup_test_app().await;

    // 1. Directory traversal in token is rejected with 400 Bad Request
    let req = Request::builder()
        .method("GET")
        .uri("/.well-known/acme-challenge/..%2F..%2Fetc%2Fpasswd")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 2. Token with illegal characters (spaces, semicolons) is rejected
    let req2 = Request::builder()
        .method("GET")
        .uri("/.well-known/acme-challenge/invalid!token@123")
        .body(Body::empty())
        .unwrap();

    let res2 = app.clone().oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::BAD_REQUEST);

    // 3. Excessively short token (< 8 chars) is rejected
    let req3 = Request::builder()
        .method("GET")
        .uri("/.well-known/acme-challenge/short")
        .body(Body::empty())
        .unwrap();

    let res3 = app.oneshot(req3).await.unwrap();
    assert_eq!(res3.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_private_ip_ssrf_rejection_in_webhooks() {
    let (app, _) = setup_test_app().await;

    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let token =
        generate_token(user_id, TenantId::from(tenant_id), "admin", TEST_SECRET, 1).unwrap();

    // 1. Private RFC 1918 Class C address (192.168.x.x) is rejected
    let payload = json!({
        "url": "http://192.168.1.1/admin-hook",
        "event_types": ["order.created"]
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/webhooks")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 2. Private RFC 1918 Class A address (10.x.x.x) is rejected
    let payload2 = json!({
        "url": "http://10.0.0.1/internal-hook",
        "event_types": ["order.created"]
    });
    let req2 = Request::builder()
        .method("POST")
        .uri("/api/v1/webhooks")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload2.to_string()))
        .unwrap();

    let res2 = app.oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_mfa_pending_token_privilege_isolation() {
    let (app, _) = setup_test_app().await;

    // Generate an intermediate token with role "superadmin_mfa_pending"
    let mfa_pending_token = generate_token(
        Uuid::new_v4(),
        TenantId(Uuid::nil()),
        "superadmin_mfa_pending",
        TEST_SECRET,
        1,
    )
    .unwrap();

    // Attempting to call protected Super Admin routes with this token MUST be rejected
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/superadmin/stats")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", mfa_pending_token),
        )
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Attempting to call tenant routes with this token MUST also be rejected
    let req2 = Request::builder()
        .method("GET")
        .uri("/api/v1/catalog/products")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", mfa_pending_token),
        )
        .body(Body::empty())
        .unwrap();

    let res2 = app.oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_password_length_enforcement() {
    let (app, state) = setup_test_app().await;

    // Create an organization first
    let org_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let org = obaf_database::entities::organization::ActiveModel {
        id: Set(org_id),
        name: Set("Security Org".to_string()),
        slug: Set("security-org".to_string()),
        preset: Set("retail".to_string()),
        active_modules: Set("[\"catalog\"]".to_string()),
        settings: Set("{}".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };
    org.insert(&state.db).await.unwrap();

    // Attempt registration with weak password (< 8 characters)
    let short_pw_payload = json!({
        "organization_id": org_id,
        "email": "user@security.org",
        "password": "123",
        "full_name": "Short Pw User",
        "role": "cashier"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/register")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(short_pw_payload.to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("at least 8 characters"));
}

#[tokio::test]
async fn test_token_logout_revocation_blacklist() {
    let (app, state) = setup_test_app().await;

    // Create an organization and user
    let org_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let org = obaf_database::entities::organization::ActiveModel {
        id: Set(org_id),
        name: Set("Revoke Org".to_string()),
        slug: Set("revoke-org".to_string()),
        preset: Set("retail".to_string()),
        active_modules: Set("[\"catalog\"]".to_string()),
        settings: Set("{}".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };
    org.insert(&state.db).await.unwrap();

    let test_password =
        std::env::var("OBAF_TEST_PASSWORD").unwrap_or_else(|_| Uuid::new_v4().to_string());

    let user = obaf_database::entities::user::ActiveModel {
        id: Set(user_id),
        organization_id: Set(org_id),
        email: Set("revokeme@security.org".to_string()),
        password_hash: Set(obaf_core::security::hash_password(&test_password).unwrap()),
        full_name: Set("Revoke User".to_string()),
        role: Set("admin".to_string()),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    };
    user.insert(&state.db).await.unwrap();

    let token = generate_token(user_id, TenantId(org_id), "admin", TEST_SECRET, 24).unwrap();

    // 1. Verify token works
    let req1 = Request::builder()
        .method("GET")
        .uri("/api/v1/auth/me")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res1 = app.clone().oneshot(req1).await.unwrap();
    assert_eq!(res1.status(), StatusCode::OK);

    // 2. Call logout endpoint to revoke the token
    let req_logout = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/logout")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res_logout = app.clone().oneshot(req_logout).await.unwrap();
    assert_eq!(res_logout.status(), StatusCode::OK);

    // 3. Verify same token is now rejected with 401 UNAUTHORIZED
    let req2 = Request::builder()
        .method("GET")
        .uri("/api/v1/auth/me")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res2 = app.oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_superadmin_logout_revocation() {
    let (app, _) = setup_test_app().await;

    let token = generate_token(
        obaf_api::superadmin::auth::SUPERADMIN_USER_ID,
        TenantId(Uuid::nil()),
        "superadmin",
        TEST_SECRET,
        24,
    )
    .unwrap();

    // 1. Verify SuperAdmin token works
    let req1 = Request::builder()
        .method("GET")
        .uri("/api/v1/superadmin/me")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res1 = app.clone().oneshot(req1).await.unwrap();
    assert_eq!(res1.status(), StatusCode::OK);

    // 2. Call SuperAdmin logout endpoint
    let req_logout = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/logout")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res_logout = app.clone().oneshot(req_logout).await.unwrap();
    assert_eq!(res_logout.status(), StatusCode::OK);

    // 3. Verify same SuperAdmin token is now rejected with 401 UNAUTHORIZED
    let req2 = Request::builder()
        .method("GET")
        .uri("/api/v1/superadmin/me")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res2 = app.oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_financial_business_logic_integrity() {
    let (app, state) = setup_test_app().await;

    let org_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let org = obaf_database::entities::organization::ActiveModel {
        id: Set(org_id),
        name: Set("Financial Org".to_string()),
        slug: Set("fin-org".to_string()),
        preset: Set("retail".to_string()),
        active_modules: Set("[\"catalog\",\"orders\",\"payments\"]".to_string()),
        settings: Set("{}".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };
    org.insert(&state.db).await.unwrap();

    let token = generate_token(Uuid::new_v4(), TenantId(org_id), "admin", TEST_SECRET, 24).unwrap();

    // 1. Attempt creating product with negative price
    let neg_prod_payload = json!({
        "name": "Negative Price Item",
        "sku": "NEG-001",
        "price": "-10.00",
        "cost_price": "5.00",
        "tax_rate": "0.1"
    });

    let req_prod = Request::builder()
        .method("POST")
        .uri("/api/v1/catalog/products")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(neg_prod_payload.to_string()))
        .unwrap();

    let res_prod = app.clone().oneshot(req_prod).await.unwrap();
    assert_eq!(res_prod.status(), StatusCode::BAD_REQUEST);

    // Create a valid product for order tests
    let valid_prod_id = Uuid::new_v4();
    let valid_prod = obaf_database::entities::product::ActiveModel {
        id: Set(valid_prod_id),
        tenant_id: Set(org_id),
        category_id: Set(None),
        sku: Set("VALID-001".to_string()),
        name: Set("Valid Widget".to_string()),
        price: Set(Decimal::new(2000, 2)), // 20.00
        cost_price: Set(Decimal::new(1000, 2)),
        tax_rate: Set(Decimal::new(1, 1)), // 0.1
        metadata: Set("{}".to_string()),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    };
    valid_prod.insert(&state.db).await.unwrap();

    // 2. Attempt creating order with negative quantity
    let neg_order_payload = json!({
        "items": [
            {
                "product_id": valid_prod_id,
                "quantity": "-2.00"
            }
        ]
    });

    let req_order = Request::builder()
        .method("POST")
        .uri("/api/v1/orders")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(neg_order_payload.to_string()))
        .unwrap();

    let res_order = app.clone().oneshot(req_order).await.unwrap();
    assert_eq!(res_order.status(), StatusCode::BAD_REQUEST);
    let bytes = to_bytes(res_order.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("strictly greater than zero"));

    // 3. Attempt recording payment with negative amount
    let neg_pay_payload = json!({
        "amount": "-50.00",
        "payment_method": "CASH"
    });

    let req_pay = Request::builder()
        .method("POST")
        .uri("/api/v1/payments")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(neg_pay_payload.to_string()))
        .unwrap();

    let res_pay = app.oneshot(req_pay).await.unwrap();
    assert_eq!(res_pay.status(), StatusCode::BAD_REQUEST);
    let bytes_pay = to_bytes(res_pay.into_body(), usize::MAX).await.unwrap();
    let body_pay: serde_json::Value = serde_json::from_slice(&bytes_pay).unwrap();
    assert!(body_pay["error"]
        .as_str()
        .unwrap()
        .contains("strictly greater than zero"));
}

#[tokio::test]
async fn test_tenant_login_brute_force_lockout() {
    let (app, state) = setup_test_app().await;

    let org_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let org = obaf_database::entities::organization::ActiveModel {
        id: Set(org_id),
        name: Set("Lockout Org".to_string()),
        slug: Set("lockout-org".to_string()),
        preset: Set("retail".to_string()),
        active_modules: Set("[\"catalog\"]".to_string()),
        settings: Set("{}".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };
    org.insert(&state.db).await.unwrap();

    let user_id = Uuid::new_v4();
    let correct_password = Uuid::new_v4().to_string();
    let user = obaf_database::entities::user::ActiveModel {
        id: Set(user_id),
        organization_id: Set(org_id),
        email: Set("target@security.org".to_string()),
        password_hash: Set(obaf_core::security::hash_password(&correct_password).unwrap()),
        full_name: Set("Target User".to_string()),
        role: Set("admin".to_string()),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    };
    user.insert(&state.db).await.unwrap();

    let wrong_payload = json!({
        "email": "target@security.org",
        "password": "wrong-password-guess"
    });

    // Fire 4 incorrect attempts (first 4 return 401 UNAUTHORIZED)
    for _ in 0..4 {
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(wrong_payload.to_string()))
            .unwrap();

        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    // 5th attempt reaches threshold and triggers lockout (429 TOO_MANY_REQUESTS)
    let req_lockout = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(wrong_payload.to_string()))
        .unwrap();

    let res_lockout = app.clone().oneshot(req_lockout).await.unwrap();
    assert_eq!(res_lockout.status(), StatusCode::TOO_MANY_REQUESTS);

    // 6th attempt is blocked immediately by check_lockout (429 TOO_MANY_REQUESTS)
    let req_locked = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(wrong_payload.to_string()))
        .unwrap();

    let res_locked = app.oneshot(req_locked).await.unwrap();
    assert_eq!(res_locked.status(), StatusCode::TOO_MANY_REQUESTS);
    let bytes = to_bytes(res_locked.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("Too many failed login attempts"));
}

#[tokio::test]
async fn test_rate_limiter_state_exhaustion_dos_protection() {
    use obaf_core::rate_limiter::TokenBucketRateLimiter;

    // Create a rate limiter with a strict maximum bound of 20 buckets
    let limiter = TokenBucketRateLimiter::with_max_buckets(10.0, 1.0, 20);

    // Simulate an attacker spraying 200 distinct spoofed client identifiers
    for i in 0..200 {
        let key = format!("spoofed_ip_{}", i);
        let _ = limiter.check_and_consume(&key, 1.0);
    }

    // After 200 distinct keys, bucket count MUST remain strictly bounded to <= 20
    assert!(
        limiter.buckets_len() <= 20,
        "Rate limiter state must not exceed configured capacity"
    );
}

#[tokio::test]
async fn test_ttl_cache_hard_capacity_eviction_dos_protection() {
    use obaf_core::cache::TtlCache;
    use std::time::Duration;

    // Cache with maximum capacity of 15 items
    let cache = TtlCache::new(Duration::from_secs(300), 15);

    // Attacker floods 100 distinct items into the cache
    for i in 0..100 {
        cache.insert(format!("key_{}", i), format!("val_{}", i));
    }

    // Cache size MUST be strictly capped at max_capacity (<= 15)
    assert!(
        cache.len() <= 15,
        "Cache entries count must strictly enforce max_capacity ceiling"
    );
}

#[tokio::test]
async fn test_query_pagination_limit_clamping_dos_protection() {
    let (app, state) = setup_test_app().await;

    let org_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let org = obaf_database::entities::organization::ActiveModel {
        id: Set(org_id),
        name: Set("Pagination Org".to_string()),
        slug: Set("page-org".to_string()),
        preset: Set("retail".to_string()),
        active_modules: Set("[\"orders\"]".to_string()),
        settings: Set("{}".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };
    org.insert(&state.db).await.unwrap();

    let token = generate_token(Uuid::new_v4(), TenantId(org_id), "admin", TEST_SECRET, 24).unwrap();

    // Attacker attempts to request an unbounded 10 million rows
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/orders?limit=10000000")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    // Must succeed normally (clamped to 200 max) without crashing or OOM
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_revoked_token_rejected_on_websocket() {
    let (app, state) = setup_test_app().await;

    let user_id = Uuid::new_v4();
    let org_id = Uuid::new_v4();
    let token = generate_token(user_id, TenantId(org_id), "admin", TEST_SECRET, 24).unwrap();

    // Revoke token immediately
    state.revoke_token(&token);

    // Attempt connecting to WebSocket with revoked token
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/ws/events?token={}", token))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_jwt_algorithm_confusion_defense() {
    let (app, _) = setup_test_app().await;

    // Attacker crafts token signed with HS512 instead of mandated HS256
    let claims = obaf_core::JwtClaims {
        sub: Uuid::new_v4(),
        tenant_id: TenantId(Uuid::new_v4()),
        role: "admin".to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
    };
    let hs512_header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS512);
    let hs512_token = jsonwebtoken::encode(
        &hs512_header,
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(TEST_SECRET.as_bytes()),
    )
    .unwrap();

    // Verify obaf_core::verify_token rejects non-HS256 algorithms
    let verify_res = obaf_core::verify_token(&hs512_token, TEST_SECRET);
    assert!(
        verify_res.is_err(),
        "Algorithm confusion attempt (HS512) must be rejected"
    );

    // API request with HS512 token must be rejected with 401 Unauthorized
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/catalog/products")
        .header(header::AUTHORIZATION, format!("Bearer {}", hs512_token))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn test_constant_time_comparison() {
    assert!(obaf_core::constant_time_compare("secret_123", "secret_123"));
    assert!(!obaf_core::constant_time_compare(
        "secret_123",
        "secret_124"
    ));
    assert!(!obaf_core::constant_time_compare("secret_123", "secret"));
    assert!(!obaf_core::constant_time_compare("short", "longer_secret"));
    assert!(obaf_core::constant_time_compare("", ""));
}

#[tokio::test]
async fn test_host_header_poisoning_rejection() {
    let (app, _) = setup_test_app().await;

    // 1. Host with control character / newline injection
    let req1 = Request::builder()
        .method("GET")
        .uri("/health")
        .header(header::HOST, "evil.com\r\nX-Injected: attack")
        .body(Body::empty())
        .unwrap_or_else(|_| {
            // Some HTTP libraries might reject building malformed headers directly; fallback to space or slash
            Request::builder()
                .method("GET")
                .uri("/health")
                .header(header::HOST, "evil.com/attack")
                .body(Body::empty())
                .unwrap()
        });

    let res1 = app.clone().oneshot(req1).await.unwrap();
    assert_eq!(res1.status(), StatusCode::BAD_REQUEST);

    // 2. Malformed X-Forwarded-Host header with illegal characters
    let req2 = Request::builder()
        .method("GET")
        .uri("/health")
        .header(header::HOST, "localhost:8992")
        .header("x-forwarded-host", "attacker.com:80<script>")
        .body(Body::empty())
        .unwrap();

    let res2 = app.clone().oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::BAD_REQUEST);

    // 3. Valid Host header accepted
    let req3 = Request::builder()
        .method("GET")
        .uri("/health")
        .header(header::HOST, "my-business-app.com:8992")
        .body(Body::empty())
        .unwrap();

    let res3 = app.oneshot(req3).await.unwrap();
    assert_eq!(res3.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_registration_privilege_escalation_and_superadmin_blocking() {
    let (app, state) = setup_test_app().await;

    let org_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let org = obaf_database::entities::organization::ActiveModel {
        id: Set(org_id),
        name: Set("Auth Test Org".to_string()),
        slug: Set("auth-test-org".to_string()),
        preset: Set("retail".to_string()),
        active_modules: Set("[\"catalog\"]".to_string()),
        settings: Set("{}".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };
    org.insert(&state.db).await.unwrap();

    // 1. Attempt to register as superadmin via tenant auth endpoint -> MUST be rejected with 403 Forbidden
    let superadmin_reg = json!({
        "organization_id": org_id,
        "email": "hacker@evil.com",
        "password": "Password123!",
        "full_name": "Hacker Admin",
        "role": "superadmin"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/register")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(superadmin_reg.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 2. Attempt to register with invalid non-existent role -> 400 Bad Request
    let invalid_role_reg = json!({
        "organization_id": org_id,
        "email": "invalid@role.com",
        "password": "Password123!",
        "full_name": "Invalid Role",
        "role": "god_mode"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/register")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(invalid_role_reg.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 3. Register initial admin for fresh organization (0 users) -> 201 Created
    let initial_admin_reg = json!({
        "organization_id": org_id,
        "email": "first_admin@auth.org",
        "password": "InitialAdminPass123!",
        "full_name": "First Admin",
        "role": "admin"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/register")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(initial_admin_reg.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let admin_token = body["token"].as_str().unwrap().to_string();

    // 4. Anonymous stranger attempts to register another admin into the established org -> 403 Forbidden
    let stranger_reg = json!({
        "organization_id": org_id,
        "email": "stranger@takeover.org",
        "password": "StrangerPass123!",
        "full_name": "Stranger",
        "role": "admin"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/register")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(stranger_reg.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 5. Registration with existing tenant admin authorization -> 201 Created
    let auth_user_reg = json!({
        "organization_id": org_id,
        "email": "staff_member@auth.org",
        "password": "StaffMemberPass123!",
        "full_name": "Staff Member",
        "role": "staff"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/register")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::from(auth_user_reg.to_string()))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_cross_tenant_usage_bola_rejection() {
    let (app, state) = setup_test_app().await;

    let org_a = Uuid::new_v4();
    let org_b = Uuid::new_v4();
    let now = chrono::Utc::now();

    for (id, name, slug) in [(org_a, "Org A", "org-a"), (org_b, "Org B", "org-b")] {
        let org = obaf_database::entities::organization::ActiveModel {
            id: Set(id),
            name: Set(name.to_string()),
            slug: Set(slug.to_string()),
            preset: Set("retail".to_string()),
            active_modules: Set("[\"orders\"]".to_string()),
            settings: Set("{}".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        };
        org.insert(&state.db).await.unwrap();
    }

    let token_a =
        generate_token(Uuid::new_v4(), TenantId(org_a), "admin", TEST_SECRET, 24).unwrap();

    // Tenant A attempts to access Tenant B's usage metrics -> 403 Forbidden
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/admin/tenants/{}/usage", org_b))
        .header(header::AUTHORIZATION, format!("Bearer {}", token_a))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // Tenant A accesses own usage metrics -> 200 OK
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/admin/tenants/{}/usage", org_a))
        .header(header::AUTHORIZATION, format!("Bearer {}", token_a))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Tenant A lists tenants -> only sees Org A
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/admin/tenants")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_a))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let list: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["id"].as_str().unwrap(), org_a.to_string());
}

#[tokio::test]
async fn test_cross_tenant_payment_and_credit_note_idor_rejection() {
    let (app, state) = setup_test_app().await;

    let org_a = Uuid::new_v4();
    let org_b = Uuid::new_v4();
    let now = chrono::Utc::now();

    for (id, name, slug) in [(org_a, "Org A", "org-a-pay"), (org_b, "Org B", "org-b-pay")] {
        let org = obaf_database::entities::organization::ActiveModel {
            id: Set(id),
            name: Set(name.to_string()),
            slug: Set(slug.to_string()),
            preset: Set("retail".to_string()),
            active_modules: Set("[\"orders\",\"payments\",\"billing\"]".to_string()),
            settings: Set("{}".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        };
        org.insert(&state.db).await.unwrap();
    }

    // Create an order in Org B
    let order_b_id = Uuid::new_v4();
    let order_b = obaf_database::entities::order::ActiveModel {
        id: Set(order_b_id),
        tenant_id: Set(org_b),
        order_number: Set("ORD-B-100".to_string()),
        customer_name: Set(Some("Alice B".to_string())),
        table_or_room: Set(None),
        status: Set("PLACED".to_string()),
        subtotal: Set(Decimal::new(10000, 2)),
        tax_total: Set(Decimal::new(500, 2)),
        discount_total: Set(Decimal::ZERO),
        grand_total: Set(Decimal::new(10500, 2)),
        metadata: Set("{}".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };
    order_b.insert(&state.db).await.unwrap();

    let token_a =
        generate_token(Uuid::new_v4(), TenantId(org_a), "admin", TEST_SECRET, 24).unwrap();

    // Tenant A attempts to record payment referencing Org B's order -> 400 Bad Request
    let pay_req = json!({
        "order_id": order_b_id,
        "amount": "50.00",
        "payment_method": "CASH"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/payments")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_a))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(pay_req.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Tenant A attempts to issue credit note referencing Org B's order -> 400 Bad Request
    let cn_req = json!({
        "order_id": order_b_id,
        "amount": "50.00",
        "tax_amount": "2.50",
        "reason": "Test IDOR"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/billing/credit-notes")
        .header(header::AUTHORIZATION, format!("Bearer {}", token_a))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(cn_req.to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_role_privilege_enforcement_on_exports_and_credit_notes() {
    let (app, state) = setup_test_app().await;

    let org_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let org = obaf_database::entities::organization::ActiveModel {
        id: Set(org_id),
        name: Set("Role Org".to_string()),
        slug: Set("role-org".to_string()),
        preset: Set("retail".to_string()),
        active_modules: Set("[\"reports\",\"billing\"]".to_string()),
        settings: Set("{}".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };
    org.insert(&state.db).await.unwrap();

    let cashier_token =
        generate_token(Uuid::new_v4(), TenantId(org_id), "cashier", TEST_SECRET, 24).unwrap();
    let admin_token =
        generate_token(Uuid::new_v4(), TenantId(org_id), "admin", TEST_SECRET, 24).unwrap();

    // 1. Cashier attempts to export customer personal data -> 403 Forbidden
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/reports/export/customers.csv")
        .header(header::AUTHORIZATION, format!("Bearer {}", cashier_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 2. Admin exports customer personal data -> 200 OK
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/reports/export/customers.csv")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. Cashier attempts to issue credit note -> 403 Forbidden
    let cn_req = json!({
        "amount": "100.00",
        "tax_amount": "5.00",
        "reason": "Unauthorized refund"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/billing/credit-notes")
        .header(header::AUTHORIZATION, format!("Bearer {}", cashier_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(cn_req.to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_cashier_shift_cross_user_close_bola_defense() {
    let (app, _state) = setup_test_app().await;

    let org_id = Uuid::new_v4();
    let cashier1_id = Uuid::new_v4();
    let cashier2_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();

    let cashier1_token =
        generate_token(cashier1_id, TenantId(org_id), "cashier", TEST_SECRET, 24).unwrap();
    let cashier2_token =
        generate_token(cashier2_id, TenantId(org_id), "cashier", TEST_SECRET, 24).unwrap();
    let admin_token = generate_token(admin_id, TenantId(org_id), "admin", TEST_SECRET, 24).unwrap();

    // 1. Cashier 1 opens shift
    let open_req = json!({
        "register_name": "Till 1",
        "opening_float": "100.00"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/pos/shifts/open")
        .header(header::AUTHORIZATION, format!("Bearer {}", cashier1_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(open_req.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let shift_id = body_json["id"].as_str().unwrap();

    // 2. Cashier 2 attempts to close Cashier 1's shift -> 403 Forbidden (BOLA/IDOR Defense)
    let close_req = json!({
        "counted_cash": "100.00",
        "notes": "Unauthorized close"
    });
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/pos/shifts/{}/close", shift_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", cashier2_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(close_req.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 3. Admin closes Cashier 1's shift -> 200 OK (Admin override allowed)
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/pos/shifts/{}/close", shift_id))
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(close_req.to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_viewer_role_strictly_forbidden_from_mutating_state() {
    let (app, _state) = setup_test_app().await;

    let org_id = Uuid::new_v4();
    let viewer_token =
        generate_token(Uuid::new_v4(), TenantId(org_id), "viewer", TEST_SECRET, 24).unwrap();

    // 1. Viewer attempts to create customer -> 403 Forbidden
    let cust_req = json!({
        "name": "Jane Doe"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/customers")
        .header(header::AUTHORIZATION, format!("Bearer {}", viewer_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(cust_req.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 2. Viewer attempts to place order -> 403 Forbidden
    let ord_req = json!({
        "items": [
            {
                "product_id": Uuid::new_v4(),
                "quantity": "1.00"
            }
        ]
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/orders")
        .header(header::AUTHORIZATION, format!("Bearer {}", viewer_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(ord_req.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 3. Viewer attempts to redeem coupon -> 403 Forbidden
    let redeem_req = json!({
        "subtotal": "50.00"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/promotions/coupons/DISCOUNT10/redeem")
        .header(header::AUTHORIZATION, format!("Bearer {}", viewer_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(redeem_req.to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_insecure_default_passwords_rejected_across_all_onboarding_endpoints() {
    let (app, _state) = setup_test_app().await;

    let superadmin_token = generate_token(
        obaf_api::superadmin::auth::SUPERADMIN_USER_ID,
        TenantId(Uuid::nil()),
        "superadmin",
        TEST_SECRET,
        24,
    )
    .unwrap();

    // 1. Superadmin Create Org without admin_password -> 400 Bad Request
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/organizations")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", superadmin_token),
        )
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "name": "No Password Org",
                "slug": "no-password-org",
                "admin_name": "Admin",
                "admin_email": "admin@nopw.org"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 2. Superadmin Create Org with short admin_password (< 8 chars) -> 400 Bad Request
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/organizations")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", superadmin_token),
        )
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "name": "Short Password Org",
                "slug": "short-password-org",
                "admin_name": "Admin",
                "admin_email": "admin@shortpw.org",
                "admin_password": "short"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 3. Admin Setup Preset without admin_password -> 400 Bad Request
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/setup/preset")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "organization_name": "No PW Preset",
                "slug": "no-pw-preset",
                "preset": "restaurant_pos",
                "admin_name": "Preset Admin",
                "admin_email": "admin@nopwpreset.org"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 4. Admin Provision Tenant without admin_password -> 400 Bad Request
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/tenants")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "organization_name": "No PW Tenant",
                "slug": "no-pw-tenant",
                "admin_name": "Tenant Admin",
                "admin_email": "admin@nopwtenant.org"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 5. Admin Provision Tenant with short password -> 400 Bad Request
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/tenants")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "organization_name": "Short PW Tenant",
                "slug": "short-pw-tenant",
                "admin_name": "Tenant Admin",
                "admin_email": "admin@shorttenant.org",
                "admin_password": "123"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_duplicate_admin_email_rejected_across_onboarding_endpoints() {
    let (app, _state) = setup_test_app().await;

    // 1. Successfully provision first tenant
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/tenants")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "organization_name": "Alpha Corp",
                "slug": "alpha-corp",
                "admin_name": "Alpha Admin",
                "admin_email": "shared_admin@corp.com",
                "admin_password": "AlphaPassword123!"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // 2. Attempt to provision second tenant with identical admin email -> 409 Conflict
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/tenants")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "organization_name": "Beta Corp",
                "slug": "beta-corp",
                "admin_name": "Beta Admin",
                "admin_email": "shared_admin@corp.com",
                "admin_password": "BetaPassword123!"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);

    // 3. Attempt to bootstrap preset with identical admin email -> 409 Conflict
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/admin/setup/preset")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "organization_name": "Gamma Corp",
                "slug": "gamma-corp",
                "preset": "restaurant_pos",
                "admin_name": "Gamma Admin",
                "admin_email": "shared_admin@corp.com",
                "admin_password": "GammaPassword123!"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_google_sso_unconfigured_audience_confusion_defense() {
    let (app, _state) = setup_test_app().await;

    // Attempting Google login with a real-looking JWT when Google SSO client ID is unconfigured -> 403 Forbidden
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/google-login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "credential": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.dummy_token_without_mock_prefix"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_database_backup_export_redaction_and_secret_filtering() {
    let (app, state) = setup_test_app().await;

    // Insert a sensitive setting (e.g. TOTP secret)
    obaf_database::set_system_setting(
        &state.db,
        "superadmin_totp_secret",
        "JBSWY3DPEHPK3PXP",
        None,
    )
    .await
    .unwrap();

    let token = generate_token(
        uuid::Uuid::new_v4(),
        TenantId(uuid::Uuid::nil()),
        "superadmin",
        &state.jwt_secret,
        1,
    )
    .unwrap();

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/superadmin/database/backup")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), 10 * 1024 * 1024)
        .await
        .unwrap();
    let backup_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Verify all users in the backup have their password_hash redacted
    let users = backup_json["users"].as_array().expect("Users array");
    for u in users {
        assert_eq!(
            u["password_hash"], "[REDACTED]",
            "Password hash must be redacted in database backup"
        );
    }

    // Verify sensitive secrets are completely filtered out from system_settings
    let settings = backup_json["system_settings"]
        .as_array()
        .expect("Settings array");
    for s in settings {
        let key = s["key"].as_str().unwrap_or("").to_lowercase();
        assert!(
            !key.contains("secret") && !key.contains("recovery_code") && !key.contains("password"),
            "Sensitive key '{}' must not appear in exported settings",
            key
        );
    }
}

#[tokio::test]
async fn test_sensitive_request_headers_layer() {
    let (app, _) = setup_test_app().await;

    // Send request with sensitive authentication and signature headers
    let req = Request::builder()
        .method("GET")
        .uri("/health")
        .header(header::AUTHORIZATION, "Bearer super-secret-jwt-token")
        .header(header::COOKIE, "session_id=ultra-confidential-cookie")
        .header("x-api-key", "prod-api-key-999")
        .header("x-obaf-signature", "sha256=abcdef1234567890")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_websocket_handshake_upgrade_and_protocols() {
    let (app, state) = setup_test_app().await;

    let user_id = Uuid::new_v4();
    let org_id = Uuid::new_v4();
    let token = generate_token(user_id, TenantId(org_id), "admin", &state.jwt_secret, 24).unwrap();

    // 1. Authenticated client sending RFC 6455 handshake headers reaches WebSocketUpgrade extractor
    // (In in-memory oneshot testing without a live TCP duplex stream, Axum returns 426 Upgrade Required)
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/ws/events")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONNECTION, "Upgrade")
        .header(header::UPGRADE, "websocket")
        .header("sec-websocket-version", "13")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UPGRADE_REQUIRED);

    // 2. Unauthenticated client sending handshake headers -> 401 Unauthorized
    let unauth_req = Request::builder()
        .method("GET")
        .uri("/api/v1/ws/events")
        .header(header::CONNECTION, "Upgrade")
        .header(header::UPGRADE, "websocket")
        .header("sec-websocket-version", "13")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .body(Body::empty())
        .unwrap();

    let unauth_res = app.oneshot(unauth_req).await.unwrap();
    assert_eq!(unauth_res.status(), StatusCode::UNAUTHORIZED);
}
