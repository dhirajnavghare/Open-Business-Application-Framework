use axum::{routing::get, Router};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::admin::admin_routes;
use crate::auth::auth_routes;
use crate::billing::billing_routes;
use crate::bookings::booking_routes;
use crate::catalog::catalog_routes;
use crate::customers::customer_routes;
use crate::docs::ApiDoc;
use crate::events::event_routes;
use crate::health::health_check;
use crate::inventory::inventory_routes;
use crate::meta::{engine_info, schema_discovery};
use crate::orders::order_routes;
use crate::payments::payment_routes;
use crate::state::SharedState;
use crate::subscriptions::subscription_routes;
use crate::webhooks::webhook_routes;

pub fn create_router(state: SharedState) -> Router {
    let openapi = ApiDoc::openapi();

    let meta_routes = Router::new()
        .route("/info", get(engine_info))
        .route("/schema", get(schema_discovery));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health check
        .route("/health", get(health_check))
        // Swagger UI & OpenAPI specification endpoint (/api/v1/openapi.json is served by SwaggerUi)
        .merge(SwaggerUi::new("/swagger-ui").url("/api/v1/openapi.json", openapi))
        // API v1 routes
        .nest("/api/v1/auth", auth_routes())
        .nest("/api/v1/customers", customer_routes())
        .nest("/api/v1/catalog", catalog_routes())
        .nest("/api/v1/inventory", inventory_routes())
        .nest("/api/v1/orders", order_routes())
        .nest("/api/v1/payments", payment_routes())
        .nest("/api/v1/bookings", booking_routes())
        .nest("/api/v1/subscriptions", subscription_routes())
        .nest("/api/v1/billing", billing_routes())
        .nest("/api/v1/webhooks", webhook_routes())
        .nest("/api/v1/ws", event_routes())
        .nest("/api/v1/meta", meta_routes)
        .nest("/api/v1/admin", admin_routes())
        // Global Middlewares
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
