pub mod dispatcher;
pub mod handlers;

use crate::state::SharedState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn webhook_routes() -> Router<SharedState> {
    Router::new()
        .route(
            "/",
            post(handlers::register_webhook).get(handlers::list_webhooks),
        )
        .route(
            "/{id}",
            get(handlers::get_webhook)
                .patch(handlers::update_webhook)
                .delete(handlers::delete_webhook),
        )
        .route("/{id}/test", post(handlers::test_webhook))
}
