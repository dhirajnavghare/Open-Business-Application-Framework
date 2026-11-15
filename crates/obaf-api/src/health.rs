use axum::{
    extract::State,
    http::header,
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

#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service is healthy and ready to serve requests", body = HealthResponse)
    )
)]
pub async fn health_check(State(state): State<SharedState>) -> Json<HealthResponse> {
    let db_status = if state.db.ping().await.is_ok() {
        "connected"
    } else {
        "unreachable"
    };

    Json(HealthResponse {
        status: "ok",
        version: state.version,
        database: db_status,
    })
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
