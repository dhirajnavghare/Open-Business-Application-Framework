pub mod handlers;

use crate::state::SharedState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn payment_routes() -> Router<SharedState> {
    Router::new()
        .route("/", post(handlers::record_payment))
        .route("/settlement", get(handlers::daily_settlement))
}
