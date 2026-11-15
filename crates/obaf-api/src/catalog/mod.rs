pub mod handlers;

use crate::state::SharedState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn catalog_routes() -> Router<SharedState> {
    Router::new()
        .route(
            "/products",
            post(handlers::create_product).get(handlers::list_products),
        )
        .route("/products/:id", get(handlers::get_product))
}
