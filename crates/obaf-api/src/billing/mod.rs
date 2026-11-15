pub mod handlers;

use crate::state::SharedState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn billing_routes() -> Router<SharedState> {
    Router::new()
        .route("/calculate-tax", post(handlers::calculate_tax))
        .route("/apply-rules", post(handlers::apply_rules))
        .route("/split-bill", post(handlers::split_bill))
        .route("/orders/:id/receipt", get(handlers::generate_receipt))
        .route(
            "/orders/:id/invoice.html",
            get(handlers::render_html_invoice),
        )
        .route(
            "/credit-notes",
            post(handlers::issue_credit_note).get(handlers::list_credit_notes),
        )
        .route("/credit-notes/:id", get(handlers::get_credit_note))
        .route(
            "/exchange-rates",
            post(handlers::set_exchange_rate).get(handlers::list_exchange_rates),
        )
        .route("/convert-currency", post(handlers::convert_currency))
}
