pub mod handlers;

use crate::state::SharedState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn billing_routes() -> Router<SharedState> {
    Router::new()
        .route("/calculate-tax", post(handlers::calculate_tax))
        .route("/split-bill", post(handlers::split_bill))
        .route("/orders/:id/receipt", get(handlers::generate_receipt))
        .route(
            "/orders/:id/invoice.html",
            get(handlers::render_html_invoice),
        )
}
