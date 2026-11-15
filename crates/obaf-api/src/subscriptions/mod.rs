pub mod handlers;

use crate::state::SharedState;
use axum::{
    routing::{get, patch, post},
    Router,
};

pub fn subscription_routes() -> Router<SharedState> {
    Router::new()
        .route(
            "/",
            post(handlers::create_subscription).get(handlers::list_subscriptions),
        )
        .route(
            "/:id",
            get(handlers::get_subscription).delete(handlers::delete_subscription),
        )
        .route("/:id/status", patch(handlers::update_status))
        .route("/:id/check-in", post(handlers::check_in_member))
}
