pub mod handlers;

use crate::state::SharedState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn supplier_routes() -> Router<SharedState> {
    Router::new()
        .route(
            "/",
            post(handlers::create_supplier).get(handlers::list_suppliers),
        )
        .route(
            "/{id}",
            get(handlers::get_supplier)
                .patch(handlers::update_supplier)
                .delete(handlers::delete_supplier),
        )
}
