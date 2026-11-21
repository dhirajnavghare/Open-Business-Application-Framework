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
