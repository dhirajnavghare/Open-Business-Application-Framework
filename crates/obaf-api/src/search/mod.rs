pub mod handlers;

use crate::state::SharedState;
use axum::{routing::get, Router};

pub fn search_routes() -> Router<SharedState> {
    Router::new().route("/", get(handlers::global_search))
}
