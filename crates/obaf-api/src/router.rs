use axum::{
    extract::{DefaultBodyLimit, State},
    http::{header, HeaderName, HeaderValue, StatusCode},
    middleware::{from_fn, from_fn_with_state, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use obaf_core::RateLimitResult;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
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
use crate::superadmin::{module_gating_middleware, serve_admin_ui, superadmin_routes};
use crate::suppliers::supplier_routes;
use crate::sync::sync_routes;
use crate::webhooks::webhook_routes;

pub fn create_router(state: SharedState) -> Router {
    let openapi = ApiDoc::openapi();

    let meta_routes = Router::new()
        .route("/info", get(engine_info))
        .route("/schema", get(schema_discovery));

    let allowed_methods = [
        axum::http::Method::GET,
        axum::http::Method::POST,
        axum::http::Method::PUT,
        axum::http::Method::PATCH,
        axum::http::Method::DELETE,
        axum::http::Method::OPTIONS,
        axum::http::Method::HEAD,
    ];

    let cors = if let Ok(origins_str) = std::env::var("OBAF_CORS_ALLOWED_ORIGINS")
        .or_else(|_| std::env::var("CORS_ALLOWED_ORIGINS"))
    {
        let origins: Vec<HeaderValue> = origins_str
            .split(',')
            .filter_map(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    HeaderValue::from_str(trimmed).ok()
                }
            })
            .collect();

        if origins.is_empty() || origins_str.trim() == "*" {
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(allowed_methods)
                .allow_headers(Any)
        } else {
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods(allowed_methods)
                .allow_headers(Any)
        }
    } else {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(allowed_methods)
            .allow_headers(Any)
    };

    Router::new()
        // Health, Metrics & AI Agent discovery endpoints
        .route("/health", get(health_check))
        .route("/live", get(live_check))
        .route("/ready", get(ready_check))
        .route("/metrics", get(metrics_check))
        .route("/llms.txt", get(llms_txt))
        // Super Admin Single Page Application UI
        .route("/admin", get(serve_admin_ui))
        .route("/superadmin", get(serve_admin_ui))
        // Let's Encrypt Automated ACME HTTP-01 Challenge Responder
        .route(
            "/.well-known/acme-challenge/{token}",
            get(crate::superadmin::ssl::acme_challenge_handler),
        )
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
        .nest("/api/v1/superadmin", superadmin_routes())
        // Global Middlewares
        .layer(from_fn(request_timeout_middleware))
        .layer(from_fn_with_state(state.clone(), module_gating_middleware))
        .layer(from_fn(request_id_and_security_middleware))
        .layer(from_fn_with_state(state.clone(), rate_limit_middleware))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .layer(cors)
        .layer(CatchPanicLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(SetSensitiveRequestHeadersLayer::new([
            header::AUTHORIZATION,
            header::COOKIE,
            HeaderName::from_static("x-api-key"),
            HeaderName::from_static("x-obaf-signature"),
        ]))
        .with_state(state)
}

async fn request_timeout_middleware(request: axum::extract::Request, next: Next) -> Response {
    let path = request.uri().path().to_string();
    // Exclude long-lived WebSockets from HTTP request timeouts
    if path.starts_with("/api/v1/ws") {
        return next.run(request).await;
    }

    match tokio::time::timeout(std::time::Duration::from_secs(30), next.run(request)).await {
        Ok(response) => response,
        Err(_) => {
            let body = serde_json::json!({
                "error": "Request Timeout",
                "message": "The server timed out waiting for the request to complete."
            });
            let mut res = Json(body).into_response();
            *res.status_mut() = StatusCode::REQUEST_TIMEOUT;
            res
        }
    }
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
        || path == "/admin"
        || path == "/superadmin"
        || path.starts_with("/swagger-ui")
        || path == "/api/v1/openapi.json"
    {
        return next.run(request).await;
    }

    // Determine client rate limiting key (Token Bucket per tenant / API key / IP)
    let client_key = if let Some(auth) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            if let Ok(claims) = obaf_core::verify_token(token, &state.jwt_secret) {
                format!("tenant:{}", claims.tenant_id.0)
            } else {
                format!("auth:{}", &token[token.len().saturating_sub(16)..])
            }
        } else {
            format!("auth:{}", &auth[auth.len().saturating_sub(16)..])
        }
    } else if let Some(api_key) = request
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
    {
        format!("apikey:{}", &api_key[api_key.len().saturating_sub(16)..])
    } else if let Some(ip) = request
        .headers()
        .get("cf-connecting-ip")
        .and_then(|v| v.to_str().ok())
    {
        format!("ip:{}", ip.trim())
    } else if let Some(ip) = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        format!(
            "ip:{}",
            ip.split(',').next_back().unwrap_or("unknown").trim()
        )
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
    // 1. Host Header Injection & Poisoning Defense
    if let Some(host_val) = request.headers().get(header::HOST) {
        if let Ok(host_str) = host_val.to_str() {
            let host_trimmed = host_str.trim();
            if host_trimmed.is_empty()
                || host_trimmed.len() > 255
                || host_trimmed.chars().any(|c| {
                    !c.is_ascii_alphanumeric()
                        && c != '.'
                        && c != '-'
                        && c != ':'
                        && c != '['
                        && c != ']'
                })
            {
                let mut res = Response::new(axum::body::Body::from("Invalid Host header"));
                *res.status_mut() = StatusCode::BAD_REQUEST;
                return res;
            }
        } else {
            let mut res = Response::new(axum::body::Body::from("Malformed Host header"));
            *res.status_mut() = StatusCode::BAD_REQUEST;
            return res;
        }
    }
    if let Some(fwd_host) = request.headers().get("x-forwarded-host") {
        if let Ok(host_str) = fwd_host.to_str() {
            let host_trimmed = host_str.trim();
            if host_trimmed.is_empty()
                || host_trimmed.len() > 255
                || host_trimmed.chars().any(|c| {
                    !c.is_ascii_alphanumeric()
                        && c != '.'
                        && c != '-'
                        && c != ':'
                        && c != '['
                        && c != ']'
                })
            {
                let mut res =
                    Response::new(axum::body::Body::from("Invalid X-Forwarded-Host header"));
                *res.status_mut() = StatusCode::BAD_REQUEST;
                return res;
            }
        }
    }

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
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self' 'unsafe-inline' https://accounts.google.com; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: https:; connect-src 'self' https://oauth2.googleapis.com https://accounts.google.com; frame-src https://accounts.google.com; object-src 'none'; base-uri 'self';",
        ),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("geolocation=(), camera=(), microphone=(), payment=(), usb=()"),
    );
    headers.insert(
        HeaderName::from_static("cross-origin-resource-policy"),
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin-allow-popups"),
    );
    response
}
