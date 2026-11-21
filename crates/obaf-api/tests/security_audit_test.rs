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
