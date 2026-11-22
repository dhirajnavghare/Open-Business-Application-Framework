pub mod handlers;

use crate::state::SharedState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn booking_routes() -> Router<SharedState> {
    Router::new()
        .route(
            "/",
            post(handlers::create_booking).get(handlers::list_bookings),
        )
        .route(
            "/{id}",
            get(handlers::get_booking).delete(handlers::delete_booking),
        )
        .route(
            "/{id}/status",
            axum::routing::patch(handlers::update_booking_status),
        )
        .route("/{id}/check-in", post(handlers::check_in_booking))
        .route("/{id}/check-out", post(handlers::check_out_booking))
}
