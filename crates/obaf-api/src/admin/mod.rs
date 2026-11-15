pub mod setup;

use crate::state::SharedState;
use axum::{routing::post, Router};

pub fn admin_routes() -> Router<SharedState> {
    Router::new().route("/setup/preset", post(setup::bootstrap_preset))
}
