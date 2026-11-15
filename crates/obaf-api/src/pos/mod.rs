pub mod handlers;

use crate::state::SharedState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn pos_routes() -> Router<SharedState> {
    Router::new()
        .route("/shifts", get(handlers::list_shifts))
        .route("/shifts/open", post(handlers::open_shift))
        .route("/shifts/current", get(handlers::get_current_shift))
        .route("/shifts/:id/close", post(handlers::close_shift))
}
