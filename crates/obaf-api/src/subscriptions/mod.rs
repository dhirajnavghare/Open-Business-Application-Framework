pub mod handlers;

use crate::state::SharedState;
use axum::{
    routing::{patch, post},
    Router,
};

pub fn subscription_routes() -> Router<SharedState> {
    Router::new()
        .route(
            "/",
            post(handlers::create_subscription).get(handlers::list_subscriptions),
        )
        .route("/:id/status", patch(handlers::update_status))
        .route("/:id/check-in", post(handlers::check_in_member))
}
