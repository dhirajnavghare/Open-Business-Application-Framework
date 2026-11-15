pub mod extractor;
pub mod handlers;

use crate::state::SharedState;
use axum::{
    routing::{get, post},
    Router,
};

pub use extractor::{AuthErrorResponse, AuthUser};

pub fn auth_routes() -> Router<SharedState> {
    Router::new()
        .route("/login", post(handlers::login))
        .route("/register", post(handlers::register))
        .route("/me", get(handlers::me))
}
