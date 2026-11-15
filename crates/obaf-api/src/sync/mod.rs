pub mod handlers;

use crate::state::SharedState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn sync_routes() -> Router<SharedState> {
    Router::new()
        .route("/push", post(handlers::sync_push))
        .route("/pull", get(handlers::sync_pull))
}
