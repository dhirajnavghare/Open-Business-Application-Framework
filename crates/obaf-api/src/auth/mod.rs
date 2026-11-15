pub mod api_keys;
pub mod extractor;
pub mod handlers;

use crate::state::SharedState;
use axum::{
    routing::{delete, get, post},
    Router,
};

pub use extractor::{AuthErrorResponse, AuthUser};

pub fn auth_routes() -> Router<SharedState> {
    Router::new()
        .route("/login", post(handlers::login))
        .route("/register", post(handlers::register))
        .route("/me", get(handlers::me))
        .route(
            "/api-keys",
            post(api_keys::create_api_key).get(api_keys::list_api_keys),
        )
        .route("/api-keys/:id", delete(api_keys::revoke_api_key))
}
