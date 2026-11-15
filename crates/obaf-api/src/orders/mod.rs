pub mod handlers;

use crate::state::SharedState;
use axum::{
    routing::{patch, post},
    Router,
};

pub fn order_routes() -> Router<SharedState> {
    Router::new()
        .route("/", post(handlers::create_order).get(handlers::list_orders))
        .route("/:id/status", patch(handlers::update_status))
}
