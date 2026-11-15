pub mod handlers;

use crate::state::SharedState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn customer_routes() -> Router<SharedState> {
    Router::new()
        .route(
            "/",
            post(handlers::create_customer).get(handlers::list_customers),
        )
        .route(
            "/:id",
            get(handlers::get_customer)
                .patch(handlers::update_customer)
                .delete(handlers::delete_customer),
        )
}
