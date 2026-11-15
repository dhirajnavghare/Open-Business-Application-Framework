pub mod audit;
pub mod setup;

use crate::state::SharedState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn admin_routes() -> Router<SharedState> {
    Router::new()
        .route("/setup/preset", post(setup::bootstrap_preset))
        .route("/audit-logs", get(audit::list_audit_logs))
}
