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

pub mod audit;
pub mod auth;
pub mod config;
pub mod middleware;
pub mod miracles;
pub mod organizations;
pub mod security;
pub mod settings;
pub mod ssl;
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
        .route("/logout", post(auth::superadmin_logout))
        .route("/config", get(auth::get_superadmin_config))
        .route("/me", get(auth::get_superadmin_me))
        // Multi-Factor Authentication (TOTP 2FA)
        .route("/mfa-verify", post(auth::superadmin_mfa_verify))
        .route("/mfa-setup", get(auth::superadmin_mfa_setup))
        .route("/mfa-activate", post(auth::superadmin_mfa_activate))
        .route("/mfa-disable", post(auth::superadmin_mfa_disable))
        // Platform Security Audit Logs
        .route("/audit-logs", get(audit::list_audit_logs))
        // Platform overview and modules catalog
        .route("/stats", get(stats::get_platform_stats))
        .route("/modules", get(organizations::list_supported_modules))
        // Organization management & API module selection
        .route(
            "/organizations",
            get(organizations::list_organizations).post(organizations::create_organization),
        )
        .route(
            "/organizations/{id}",
            get(organizations::get_organization)
                .put(organizations::update_organization)
                .delete(organizations::delete_organization),
        )
        // Platform Settings & Configuration
        .route(
            "/settings",
            get(settings::get_platform_settings).put(settings::update_platform_settings),
        )
        // Domain & Let's Encrypt Automated SSL
        .route("/ssl", get(ssl::get_ssl_status))
        .route("/ssl/configure-domain", post(ssl::configure_domain))
        .route("/ssl/provision", post(ssl::provision_ssl))
        .route("/ssl/renew", post(ssl::renew_ssl))
        .route("/ssl/proxy-config", get(ssl::get_proxy_config))
        // Super Admin Miracle Power Tools
        .route(
            "/organizations/{id}/impersonate",
            post(miracles::impersonate_tenant_admin),
        )
        .route(
            "/system/maintenance",
            get(miracles::get_maintenance_status).post(miracles::set_maintenance_mode),
        )
        .route(
            "/system/emergency-lockdown",
            post(miracles::set_emergency_lockdown),
        )
        .route("/system/lockdown", post(miracles::set_emergency_lockdown))
        .route("/database/backup", get(miracles::download_database_backup))
        .route("/database/optimize", post(miracles::optimize_database))
        .route("/system/flush-cache", post(miracles::flush_system_cache))
        .route(
            "/system/announcement",
            get(miracles::get_global_announcement).post(miracles::set_global_announcement),
        )
}
