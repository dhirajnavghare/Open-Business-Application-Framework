// Copyright 2026 Dhiraj Navghare
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub mod auth;
pub mod config;
pub mod middleware;
pub mod organizations;
pub mod settings;
pub mod stats;
pub mod ui;

pub use config::SuperAdminConfig;
pub use middleware::module_gating_middleware;
pub use ui::serve_admin_ui;

use crate::state::SharedState;
use axum::{
    routing::{get, post},
    Router,
};

/// Mount all Platform Super Administrator REST endpoints.
pub fn superadmin_routes() -> Router<SharedState> {
    Router::new()
        // Public authentication and configuration
        .route("/login", post(auth::superadmin_login))
        .route("/google-login", post(auth::superadmin_google_login))
        .route("/config", get(auth::get_superadmin_config))
        .route("/me", get(auth::get_superadmin_me))
        // Platform overview and modules catalog
        .route("/stats", get(stats::get_platform_stats))
        .route("/modules", get(organizations::list_supported_modules))
        // Organization management & API module selection
        .route(
            "/organizations",
            get(organizations::list_organizations).post(organizations::create_organization),
        )
        .route(
            "/organizations/:id",
            get(organizations::get_organization)
                .put(organizations::update_organization)
                .delete(organizations::delete_organization),
        )
        // Platform Settings & Configuration
        .route(
            "/settings",
            get(settings::get_platform_settings).put(settings::update_platform_settings),
        )
}
