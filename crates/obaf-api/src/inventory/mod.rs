pub mod handlers;

use crate::state::SharedState;
use axum::{routing::post, Router};

pub fn inventory_routes() -> Router<SharedState> {
    Router::new().route(
        "/movements",
        post(handlers::record_movement).get(handlers::list_movements),
    )
}
