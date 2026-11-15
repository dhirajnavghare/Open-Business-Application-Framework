pub mod audit;
pub mod setup;
pub mod tenants;

use crate::state::SharedState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn admin_routes() -> Router<SharedState> {
    Router::new()
        .route("/setup/preset", post(setup::bootstrap_preset))
        .route("/audit-logs", get(audit::list_audit_logs))
        .route(
            "/tenants",
            post(tenants::provision_tenant).get(tenants::list_tenants),
        )
        .route("/tenants/:id/usage", get(tenants::get_tenant_usage))
}
