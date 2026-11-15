pub mod handlers;

use crate::state::SharedState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn inventory_routes() -> Router<SharedState> {
    Router::new()
        .route(
            "/movements",
            post(handlers::record_movement).get(handlers::list_movements),
        )
        .route("/balance/:product_id", get(handlers::get_stock_balance))
        .route(
            "/locations",
            post(handlers::create_stock_location).get(handlers::list_stock_locations),
        )
        .route(
            "/locations/:id",
            get(handlers::get_stock_location)
                .patch(handlers::update_stock_location)
                .delete(handlers::delete_stock_location),
        )
}
