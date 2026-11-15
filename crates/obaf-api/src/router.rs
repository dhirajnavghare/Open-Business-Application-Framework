use axum::{
    extract::State,
    http::{header, HeaderName, HeaderValue, StatusCode},
    middleware::{from_fn, from_fn_with_state, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use obaf_core::RateLimitResult;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::admin::admin_routes;
use crate::auth::auth_routes;
use crate::billing::billing_routes;
use crate::bookings::booking_routes;
use crate::catalog::catalog_routes;
use crate::customers::customer_routes;
use crate::docs::ApiDoc;
use crate::events::event_routes;
use crate::health::{health_check, live_check, llms_txt, metrics_check, ready_check};
use crate::inventory::inventory_routes;
use crate::meta::{engine_info, schema_discovery};
use crate::orders::order_routes;
use crate::payments::payment_routes;
use crate::pos::pos_routes;
use crate::procurement::procurement_routes;
use crate::promotions::promotion_routes;
use crate::reports::report_routes;
use crate::search::search_routes;
use crate::state::SharedState;
use crate::subscriptions::subscription_routes;
use crate::suppliers::supplier_routes;
use crate::sync::sync_routes;
use crate::webhooks::webhook_routes;

pub fn create_router(state: SharedState) -> Router {
    let openapi = ApiDoc::openapi();

    let meta_routes = Router::new()
        .route("/info", get(engine_info))
        .route("/schema", get(schema_discovery));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health, Metrics & AI Agent discovery endpoints
        .route("/health", get(health_check))
        .route("/live", get(live_check))
        .route("/ready", get(ready_check))
        .route("/metrics", get(metrics_check))
        .route("/llms.txt", get(llms_txt))
        // Swagger UI & OpenAPI specification endpoint (/api/v1/openapi.json is served by SwaggerUi)
        .merge(SwaggerUi::new("/swagger-ui").url("/api/v1/openapi.json", openapi))
        // API v1 routes
        .nest("/api/v1/auth", auth_routes())
        .nest("/api/v1/customers", customer_routes())
        .nest("/api/v1/suppliers", supplier_routes())
        .nest("/api/v1/catalog", catalog_routes())
        .nest("/api/v1/inventory", inventory_routes())
        .nest("/api/v1/orders", order_routes())
        .nest("/api/v1/payments", payment_routes())
        .nest("/api/v1/bookings", booking_routes())
        .nest("/api/v1/subscriptions", subscription_routes())
        .nest("/api/v1/billing", billing_routes())
        .nest("/api/v1/pos", pos_routes())
        .nest("/api/v1/procurement", procurement_routes())
        .nest("/api/v1/promotions", promotion_routes())
        .nest("/api/v1/reports", report_routes())
        .nest("/api/v1/webhooks", webhook_routes())
        .nest("/api/v1/ws", event_routes())
        .nest("/api/v1/search", search_routes())
        .nest("/api/v1/meta", meta_routes)
        .nest("/api/v1/sync", sync_routes())
        .nest("/api/v1/admin", admin_routes())
        // Global Middlewares
        .layer(from_fn(request_id_and_security_middleware))
        .layer(from_fn_with_state(state.clone(), rate_limit_middleware))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn rate_limit_middleware(
    State(state): State<SharedState>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    // Exclude health, metrics, documentation, and metadata endpoints from rate limiting
    if path == "/health"
        || path == "/live"
        || path == "/ready"
        || path == "/metrics"
        || path == "/llms.txt"
        || path.starts_with("/swagger-ui")
        || path == "/api/v1/openapi.json"
    {
        return next.run(request).await;
    }

    // Determine client rate limiting key (Token Bucket per client/tenant)
    let client_key = if let Some(auth) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        format!("auth:{}", &auth[..auth.len().min(32)])
    } else if let Some(ip) = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        format!("ip:{}", ip.split(',').next().unwrap_or("unknown").trim())
    } else if let Some(ip) = request
        .headers()
        .get("cf-connecting-ip")
        .and_then(|v| v.to_str().ok())
    {
        format!("ip:{}", ip.trim())
    } else {
        "client:default".to_string()
    };

    match state.rate_limiter.check_and_consume(&client_key, 1.0) {
        RateLimitResult::Allowed { remaining } => {
            let mut response = next.run(request).await;
            if let Ok(rem_val) = HeaderValue::from_str(&format!("{:.0}", remaining)) {
                response
                    .headers_mut()
                    .insert(HeaderName::from_static("x-ratelimit-remaining"), rem_val);
            }
            response
        }
        RateLimitResult::Rejected { retry_after_secs } => {
            let retry_secs = (retry_after_secs.ceil() as u64).max(1);
            let body = serde_json::json!({
                "error": "Too Many Requests",
                "message": "Rate limit exceeded. Please retry after the cooling window.",
                "retry_after_secs": retry_secs
            });
            let mut response = Json(body).into_response();
            *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
            let headers = response.headers_mut();
            if let Ok(retry_val) = HeaderValue::from_str(&retry_secs.to_string()) {
                headers.insert(header::RETRY_AFTER, retry_val);
            }
            headers.insert(
                HeaderName::from_static("x-ratelimit-remaining"),
                HeaderValue::from_static("0"),
            );
            response
        }
    }
}

async fn request_id_and_security_middleware(
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let req_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let req_id_header =
        HeaderValue::from_str(&req_id).unwrap_or_else(|_| HeaderValue::from_static("obaf-trace"));

    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Distributed tracing request correlation ID (System Design Primer)
    headers.insert(HeaderName::from_static("x-request-id"), req_id_header);

    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        HeaderName::from_static("x-xss-protection"),
        HeaderValue::from_static("1; mode=block"),
    );
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    response
}
