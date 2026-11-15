use axum::{extract::State, Json};
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
