pub mod handlers;

use axum::{
    routing::{get, post},
    Router,
};

use crate::state::SharedState;
use handlers::{
    cancel_purchase_order, create_purchase_order, get_purchase_order, list_purchase_orders,
    receive_purchase_order,
};

pub fn procurement_routes() -> Router<SharedState> {
    Router::new()
        .route(
            "/purchase-orders",
            post(create_purchase_order).get(list_purchase_orders),
        )
        .route("/purchase-orders/:id", get(get_purchase_order))
        .route("/purchase-orders/:id/receive", post(receive_purchase_order))
        .route("/purchase-orders/:id/cancel", post(cancel_purchase_order))
}
