use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::SharedState;

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub database: &'static str,
}

#[derive(Serialize, ToSchema)]
pub struct LivenessResponse {
    pub status: &'static str,
    pub timestamp: String,
}

#[derive(Serialize, ToSchema)]
pub struct ReadinessResponse {
    pub status: &'static str,
    pub database: &'static str,
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service is healthy and ready to serve requests", body = HealthResponse),
        (status = 503, description = "Service is degraded (database unreachable)", body = HealthResponse)
    )
)]
pub async fn health_check(State(state): State<SharedState>) -> Response {
    let db_ok = state.db.ping().await.is_ok();
    let status_code = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let body = Json(HealthResponse {
        status: if db_ok { "ok" } else { "degraded" },
        version: state.version,
        database: if db_ok { "connected" } else { "unreachable" },
    });

    (status_code, body).into_response()
}

#[utoipa::path(
    get,
    path = "/live",
    tag = "Health",
    responses(
        (status = 200, description = "Liveness probe: verifies process is responsive and not deadlocked", body = LivenessResponse)
    )
)]
pub async fn live_check() -> Json<LivenessResponse> {
    Json(LivenessResponse {
        status: "alive",
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

#[utoipa::path(
    get,
    path = "/ready",
    tag = "Health",
    responses(
        (status = 200, description = "Readiness probe: verifies service can accept client traffic and connect to database", body = ReadinessResponse),
        (status = 503, description = "Service not ready to accept traffic", body = ReadinessResponse)
    )
)]
pub async fn ready_check(State(state): State<SharedState>) -> Response {
    let db_ok = state.db.ping().await.is_ok();
    let status_code = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let body = Json(ReadinessResponse {
        status: if db_ok { "ready" } else { "not_ready" },
        database: if db_ok { "connected" } else { "unreachable" },
    });

    (status_code, body).into_response()
}

#[utoipa::path(
    get,
    path = "/metrics",
    tag = "Health",
    responses(
        (status = 200, description = "Prometheus-compatible operational metrics", content_type = "text/plain")
    )
)]
pub async fn metrics_check(State(state): State<SharedState>) -> Response {
    let db_ok = if state.db.ping().await.is_ok() { 1 } else { 0 };
    let body = format!(
        "# HELP obaf_up Service status (1 = up)\n\
         # TYPE obaf_up gauge\n\
         obaf_up 1\n\
         # HELP obaf_database_connected Database connection health\n\
         # TYPE obaf_database_connected gauge\n\
         obaf_database_connected {}\n\
         # HELP obaf_info Framework build info\n\
         # TYPE obaf_info gauge\n\
         obaf_info{{version=\"{}\"}} 1\n",
        db_ok, state.version
    );

    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
        .into_response()
}

#[utoipa::path(
    get,
    path = "/llms.txt",
    tag = "Health",
    responses(
        (status = 200, description = "AI Agent context and documentation standard file conforming to /llms.txt", content_type = "text/markdown")
    )
)]
pub async fn llms_txt() -> Response {
    let content = include_str!("../llms.txt");
    (
        [(header::CONTENT_TYPE, "text/markdown; charset=utf-8")],
        content,
    )
        .into_response()
}
