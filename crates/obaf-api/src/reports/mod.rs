pub mod handlers;

use crate::state::SharedState;
use axum::{routing::get, Router};

pub fn report_routes() -> Router<SharedState> {
    Router::new()
        .route("/dashboard", get(handlers::get_dashboard_metrics))
        .route("/kitchen-prep", get(handlers::get_kitchen_prep_sheet))
        .route(
            "/sales-by-payment-method",
            get(handlers::get_sales_by_payment_method),
        )
        .route("/export/orders.csv", get(handlers::export_orders_csv))
        .route("/export/customers.csv", get(handlers::export_customers_csv))
        .route("/export/inventory.csv", get(handlers::export_inventory_csv))
}
