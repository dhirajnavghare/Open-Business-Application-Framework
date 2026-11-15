pub mod handlers;

use crate::state::SharedState;
use axum::{routing::post, Router};

pub fn booking_routes() -> Router<SharedState> {
    Router::new()
        .route(
            "/",
            post(handlers::create_booking).get(handlers::list_bookings),
        )
        .route("/:id/check-in", post(handlers::check_in_booking))
        .route("/:id/check-out", post(handlers::check_out_booking))
}
