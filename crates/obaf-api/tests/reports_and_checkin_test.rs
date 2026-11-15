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
async fn test_reports_and_attendance_checkin() {
    let db = establish_connection("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    run_migrations(&db).await.expect("Failed to run migrations");

    let jwt_secret = "test-secret-reports-12345";
    let state = Arc::new(AppState::new(db, jwt_secret));
    let app = create_router(state);

    let tenant = Uuid::new_v4();
    let user = Uuid::new_v4();
    let token = generate_token(user, TenantId::from(tenant), "admin", jwt_secret, 1)
        .expect("Failed to generate token");

    // 1. Create a customer
    let customer_req = Request::builder()
        .method("POST")
        .uri("/api/v1/customers")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "name": "Member Alice",
                "email": "alice@gym.test"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(customer_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let cust_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let cust_json: serde_json::Value = serde_json::from_slice(&cust_bytes).unwrap();
    let customer_id = cust_json["id"].as_str().unwrap();

    // 2. Create an active subscription
    let now = chrono::Utc::now();
    let sub_req = Request::builder()
        .method("POST")
        .uri("/api/v1/subscriptions")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "customer_id": customer_id,
                "plan_name": "Gold Gym Annual",
                "start_date": now.to_rfc3339(),
                "end_date": (now + chrono::Duration::days(365)).to_rfc3339(),
                "amount": "1200.00",
                "billing_cycle": "ANNUAL"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(sub_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let sub_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let sub_json: serde_json::Value = serde_json::from_slice(&sub_bytes).unwrap();
    let subscription_id = sub_json["id"].as_str().unwrap();

    // 3. Test Subscription Attendance Check-in
    let checkin_req = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/v1/subscriptions/{}/check-in",
            subscription_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(checkin_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let checkin_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let checkin_json: serde_json::Value = serde_json::from_slice(&checkin_bytes).unwrap();
    assert_eq!(checkin_json["status"], "checked_in");
    assert_eq!(checkin_json["total_check_ins"], 1);

    // 4. Create and Check-In / Check-Out a Booking (e.g. Visitor Pass or Table)
    let booking_req = Request::builder()
        .method("POST")
        .uri("/api/v1/bookings")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "resource_name": "Visitor Gate Pass #101",
                "start_time": now.to_rfc3339(),
                "end_time": (now + chrono::Duration::hours(2)).to_rfc3339(),
                "notes": "Contractor inspection"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(booking_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let b_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let b_json: serde_json::Value = serde_json::from_slice(&b_bytes).unwrap();
    let booking_id = b_json["id"].as_str().unwrap();

    // Check-in booking
    let b_checkin = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/bookings/{}/check-in", booking_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(b_checkin).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Check-out booking
    let b_checkout = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/bookings/{}/check-out", booking_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(b_checkout).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 5. Test Reports Dashboard
    let dash_req = Request::builder()
        .method("GET")
        .uri("/api/v1/reports/dashboard")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(dash_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let dash_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let dash_json: serde_json::Value = serde_json::from_slice(&dash_bytes).unwrap();

    assert_eq!(dash_json["total_customers"], 1);
    assert_eq!(dash_json["total_bookings"], 1);
    assert_eq!(dash_json["active_subscriptions"], 1);
}
