pub mod ws;

use crate::state::SharedState;
use axum::{routing::get, Router};

pub fn event_routes() -> Router<SharedState> {
    Router::new().route("/events", get(ws::ws_handler))
}
