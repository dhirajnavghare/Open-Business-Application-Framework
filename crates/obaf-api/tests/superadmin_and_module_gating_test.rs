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
use obaf_database::{migration::run_migrations, pool::establish_connection};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn test_superadmin_ui_serving() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    run_migrations(&db).await.unwrap();

    let state = Arc::new(AppState::new(db, "test_secret_32_characters_minimum_len"));
    let app = create_router(state.clone());

    // 1. Verify GET /admin serves the self-contained SPA
    let req = Request::builder()
        .method("GET")
        .uri("/admin")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res
        .headers()
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .contains("text/html"));

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("OBAF Super Admin Console"));
    assert!(html.contains("Select Supported APIs & Modules"));
    assert!(html.contains("google-login"));

    // 2. Verify GET /superadmin alias also serves the SPA
    let req2 = Request::builder()
        .method("GET")
        .uri("/superadmin")
        .body(Body::empty())
        .unwrap();

    let res2 = app.oneshot(req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_superadmin_authentication_and_google_sso() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    run_migrations(&db).await.unwrap();

    let state = Arc::new(AppState::new(db, "test_secret_32_characters_minimum_len"));
    let app = create_router(state.clone());

    // 1. Public config check
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/superadmin/config")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let cfg: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(cfg["superadmin_email"], "admin@obaf.local");

    // 2. Login with incorrect password fails (401)
    let bad_pw_body = json!({
        "email": "admin@obaf.local",
        "password": "wrongpassword"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(bad_pw_body.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 3. Login with correct password succeeds (200)
    let good_pw_body = json!({
        "email": "admin@obaf.local",
        "password": "admin123"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(good_pw_body.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let login_resp: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(login_resp["token"].as_str().is_some());
    assert_eq!(login_resp["user"]["role"], "superadmin");
    assert_eq!(login_resp["user"]["email"], "admin@obaf.local");
    let superadmin_token = login_resp["token"].as_str().unwrap().to_string();

    // 4. Verify /me endpoint with superadmin token
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/superadmin/me")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", superadmin_token),
        )
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let me: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(me["role"], "superadmin");

    // 5. Google SSO login with unauthorized email fails (401)
    let evil_google_body = json!({
        "credential": "mock_google_token:intruder@evil.com"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/google-login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(evil_google_body.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 6. Google SSO login with authorized Super Admin email succeeds (200)
    let good_google_body = json!({
        "credential": "mock_google_token:admin@obaf.local"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/google-login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(good_google_body.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let google_login_resp: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(google_login_resp["token"].as_str().is_some());
    assert_eq!(google_login_resp["user"]["role"], "superadmin");
}

#[tokio::test]
async fn test_organization_management_and_module_gating() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    run_migrations(&db).await.unwrap();

    let state = Arc::new(AppState::new(db, "test_secret_32_characters_minimum_len"));
    let app = create_router(state.clone());

    // 1. Super Admin login
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
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let login_resp: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let superadmin_token = login_resp["token"].as_str().unwrap().to_string();

    // 2. Query module catalog
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/superadmin/modules")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", superadmin_token),
        )
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let modules: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(modules.len(), 15);
    assert!(modules.iter().any(|m| m["id"] == "catalog"));
    assert!(modules.iter().any(|m| m["id"] == "bookings"));

    // 3. Super Admin creates an organization with ONLY ["catalog", "orders"] enabled
    let create_org_body = json!({
        "name": "Acme Retail Ltd",
        "slug": "acme-retail",
        "preset": "retail_grocery",
        "active_modules": ["catalog", "orders"],
        "admin_name": "Acme Admin",
        "admin_email": "admin@acmeretail.com",
        "admin_password": "password123"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/organizations")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", superadmin_token),
        )
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(create_org_body.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let created_org: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let org_id = created_org["id"].as_str().unwrap().to_string();
    assert_eq!(created_org["slug"], "acme-retail");
    assert_eq!(created_org["active_modules"], json!(["catalog", "orders"]));

    // 4. Verify platform stats reflects the newly created organization
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/superadmin/stats")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", superadmin_token),
        )
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let stats: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(stats["total_organizations"].as_u64().unwrap() >= 1);
    assert!(stats["total_users"].as_u64().unwrap() >= 1);

    // 5. Tenant admin logs in to obtain tenant JWT token
    let tenant_login_body = json!({
        "email": "admin@acmeretail.com",
        "password": "password123"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(tenant_login_body.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let tenant_auth: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let tenant_token = tenant_auth["token"].as_str().unwrap().to_string();

    // 6. Test Allowed Module: Tenant requests /api/v1/catalog/products
    // "catalog" is in active_modules, so this should pass module gating!
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/catalog/products")
        .header(header::AUTHORIZATION, format!("Bearer {}", tenant_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 7. Test Disabled Module: Tenant requests /api/v1/bookings
    // "bookings" is NOT in active_modules, so module gating MUST return 403 Forbidden!
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/bookings")
        .header(header::AUTHORIZATION, format!("Bearer {}", tenant_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let err_body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(err_body["code"], "MODULE_DISABLED");
    assert_eq!(err_body["module"], "bookings");

    // 8. Super Admin bypasses module gating
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/bookings")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", superadmin_token),
        )
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    // Superadmin is not rejected by MODULE_DISABLED (status is OK)
    assert_eq!(res.status(), StatusCode::OK);

    // 9. Super Admin dynamically selects/updates supported APIs for the organization
    // Enabling "bookings" module for Acme Retail Ltd!
    let update_body = json!({
        "active_modules": ["catalog", "orders", "bookings"]
    });
    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/superadmin/organizations/{}", org_id))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", superadmin_token),
        )
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(update_body.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let updated_org: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        updated_org["active_modules"],
        json!(["catalog", "orders", "bookings"])
    );

    // 10. Tenant requests /api/v1/bookings again
    // Now that Super Admin enabled "bookings", it passes immediately without 403!
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/bookings")
        .header(header::AUTHORIZATION, format!("Bearer {}", tenant_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 11. Super Admin deletes the organization
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/superadmin/organizations/{}", org_id))
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", superadmin_token),
        )
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_superadmin_platform_settings_and_persistent_jwt() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    run_migrations(&db).await.unwrap();

    // 1. Verify zero-config persistent JWT secret generation and caching
    let jwt_secret_1 = obaf_database::get_or_create_jwt_secret(&db).await.unwrap();
    assert_eq!(jwt_secret_1.len(), 64);

    let jwt_secret_2 = obaf_database::get_or_create_jwt_secret(&db).await.unwrap();
    assert_eq!(jwt_secret_1, jwt_secret_2);

    let state = Arc::new(AppState::new(db, &jwt_secret_1));
    let app = create_router(state.clone());

    // 2. Login as Super Admin
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
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let login_resp: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let superadmin_token = login_resp["token"].as_str().unwrap();

    // 3. Fetch default platform settings
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/superadmin/settings")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", superadmin_token),
        )
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let settings: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        settings["platform_name"],
        "Open Business Application Framework"
    );
    assert_eq!(settings["default_currency"], "USD");
    assert_eq!(settings["token_expiry_hours"], 24);
    assert_eq!(settings["superadmin_email"], "admin@obaf.local");

    // 4. Update platform settings via Super Admin API
    let update_body = json!({
        "platform_name": "Acme SaaS Suite",
        "default_currency": "EUR",
        "token_expiry_hours": 48,
        "google_client_id": "test-google-id-12345.apps.googleusercontent.com"
    });
    let req = Request::builder()
        .method("PUT")
        .uri("/api/v1/superadmin/settings")
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", superadmin_token),
        )
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(update_body.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let updated: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(updated["platform_name"], "Acme SaaS Suite");
    assert_eq!(updated["default_currency"], "EUR");
    assert_eq!(updated["token_expiry_hours"], 48);
    assert_eq!(
        updated["google_client_id"],
        "test-google-id-12345.apps.googleusercontent.com"
    );
    assert_eq!(updated["google_sso_active"], true);

    // 5. Verify public config now reflects dynamically configured Google SSO
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/superadmin/config")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let cfg: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(cfg["google_auth_enabled"], true);
    assert_eq!(
        cfg["google_client_id"],
        "test-google-id-12345.apps.googleusercontent.com"
    );

    // 6. Non-superadmin requests to settings are rejected (403 Forbidden)
    let non_admin_token = obaf_core::generate_token(
        uuid::Uuid::new_v4(),
        obaf_core::TenantId(uuid::Uuid::new_v4()),
        "admin",
        &jwt_secret_1,
        24,
    )
    .unwrap();

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/superadmin/settings")
        .header(header::AUTHORIZATION, format!("Bearer {}", non_admin_token))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_superadmin_brute_force_lockout() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    run_migrations(&db).await.unwrap();

    let state = Arc::new(AppState::new(db, "test_secret_32_characters_minimum_len"));
    let app = create_router(state.clone());

    let bad_req = json!({
        "email": "admin@obaf.local",
        "password": "wrongpassword123"
    });

    // 4 failed attempts should return 401 Unauthorized
    for _ in 0..4 {
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/superadmin/login")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-forwarded-for", "198.51.100.22")
            .body(Body::from(bad_req.to_string()))
            .unwrap();

        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    // 5th failed attempt triggers lockout (429 Too Many Requests)
    let req5 = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/login")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", "198.51.100.22")
        .body(Body::from(bad_req.to_string()))
        .unwrap();

    let res5 = app.clone().oneshot(req5).await.unwrap();
    assert_eq!(res5.status(), StatusCode::TOO_MANY_REQUESTS);
    let bytes = to_bytes(res5.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("Too many failed login attempts"));

    // Even with the right password, caller is locked out
    let good_req = json!({
        "email": "admin@obaf.local",
        "password": "admin123"
    });
    let req_locked = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/login")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", "198.51.100.22")
        .body(Body::from(good_req.to_string()))
        .unwrap();

    let res_locked = app.clone().oneshot(req_locked).await.unwrap();
    assert_eq!(res_locked.status(), StatusCode::TOO_MANY_REQUESTS);

    // Meanwhile, a different IP address is NOT locked out
    let req_diff_ip = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/login")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", "198.51.100.99")
        .body(Body::from(good_req.to_string()))
        .unwrap();

    let res_diff_ip = app.oneshot(req_diff_ip).await.unwrap();
    assert_eq!(res_diff_ip.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_superadmin_ip_whitelisting_gate() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    run_migrations(&db).await.unwrap();

    let state = Arc::new(AppState::new(
        db.clone(),
        "test_secret_32_characters_minimum_len",
    ));
    let app = create_router(state.clone());

    // Configure IP Whitelist via system settings
    obaf_database::set_system_setting(
        &db,
        "superadmin_allowed_ips",
        "10.0.0.0/8, 192.168.1.100",
        Some("Allowed Superadmin IPs"),
    )
    .await
    .unwrap();

    let good_login = json!({
        "email": "admin@obaf.local",
        "password": "admin123"
    });

    // 1. IP not in whitelist is blocked with 403 Forbidden
    let req_blocked = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/login")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", "203.0.113.5")
        .body(Body::from(good_login.to_string()))
        .unwrap();

    let res_blocked = app.clone().oneshot(req_blocked).await.unwrap();
    assert_eq!(res_blocked.status(), StatusCode::FORBIDDEN);
    let bytes = to_bytes(res_blocked.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body["error"].as_str().unwrap().contains("Access Denied"));

    // 2. IP in CIDR subnet (10.0.0.0/8) is permitted
    let req_cidr_allowed = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/login")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", "10.24.55.101")
        .body(Body::from(good_login.to_string()))
        .unwrap();

    let res_cidr = app.clone().oneshot(req_cidr_allowed).await.unwrap();
    assert_eq!(res_cidr.status(), StatusCode::OK);

    // 3. Exact single IP (192.168.1.100) is permitted
    let req_exact_allowed = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/login")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", "192.168.1.100")
        .body(Body::from(good_login.to_string()))
        .unwrap();

    let res_exact = app.oneshot(req_exact_allowed).await.unwrap();
    assert_eq!(res_exact.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_superadmin_mfa_totp_setup_activation_and_recovery() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    run_migrations(&db).await.unwrap();

    let state = Arc::new(AppState::new(
        db.clone(),
        "test_secret_32_characters_minimum_len",
    ));
    let app = create_router(state.clone());

    // 1. Initial login to get Super Admin Bearer token
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
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let login_res: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let admin_token = login_res["token"].as_str().unwrap();

    // 2. Request MFA Setup
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/superadmin/mfa-setup")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let setup_res: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    let totp_secret = setup_res["secret"].as_str().unwrap().to_string();
    let recovery_codes = setup_res["recovery_codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert_eq!(recovery_codes.len(), 6);

    // 3. Confirm & Activate MFA with generated TOTP code
    let now_ts = chrono::Utc::now().timestamp() as u64;
    let valid_code = obaf_core::compute_totp(&totp_secret, now_ts, 30).unwrap();

    let activate_payload = json!({
        "secret": totp_secret,
        "code": valid_code,
        "recovery_codes": recovery_codes
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/mfa-activate")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(activate_payload.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 4. Now attempt login: Must return MFA challenge (mfa_required: true)
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(login_body.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let challenge_res: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(challenge_res["mfa_required"], true);
    let temp_token = challenge_res["temp_token"].as_str().unwrap();

    // 5. Complete MFA verification with TOTP code
    let now_ts2 = chrono::Utc::now().timestamp() as u64;
    let fresh_code = obaf_core::compute_totp(&totp_secret, now_ts2, 30).unwrap();

    let verify_payload = json!({
        "temp_token": temp_token,
        "code": fresh_code
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/mfa-verify")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(verify_payload.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let full_auth: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(full_auth["token"].as_str().is_some());
    assert_eq!(full_auth["user"]["role"], "superadmin");

    // 6. Test logging in using Emergency Recovery Code
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(login_body.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let challenge_res2: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let temp_token2 = challenge_res2["temp_token"].as_str().unwrap();

    let recovery_code = &recovery_codes[0];
    let recovery_verify_payload = json!({
        "temp_token": temp_token2,
        "code": recovery_code
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/mfa-verify")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(recovery_verify_payload.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 7. Verify security audit logs recorded these events
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/superadmin/audit-logs")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let audit_logs: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
    assert!(!audit_logs.is_empty());
    let actions: Vec<&str> = audit_logs
        .iter()
        .map(|l| l["action"].as_str().unwrap())
        .collect();
    assert!(actions.contains(&"SUPERADMIN_MFA_ACTIVATED"));
    assert!(actions.contains(&"SUPERADMIN_MFA_VERIFIED"));

    // 8. Disable MFA
    let now_ts3 = chrono::Utc::now().timestamp() as u64;
    let disable_code = obaf_core::compute_totp(&totp_secret, now_ts3, 30).unwrap();
    let disable_payload = json!({ "code": disable_code });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/mfa-disable")
        .header(header::AUTHORIZATION, format!("Bearer {}", admin_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(disable_payload.to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 9. Subsequent login directly succeeds without MFA challenge
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/superadmin/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(login_body.to_string()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let login_res_final: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(login_res_final["mfa_required"].as_bool(), Some(false));
    assert!(login_res_final["token"].as_str().is_some());
}
