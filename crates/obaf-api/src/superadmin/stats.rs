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

use axum::{extract::State, http::StatusCode, Json};
use obaf_database::entities::{order, organization, product, user};
use sea_orm::{ConnectionTrait, DbBackend, EntityTrait, PaginatorTrait};
use serde::Serialize;
use utoipa::ToSchema;

use crate::auth::extractor::AuthUser;
use crate::state::SharedState;
use crate::superadmin::organizations::{ErrorResponse, ALL_MODULES};

#[derive(Debug, Serialize, ToSchema)]
pub struct PlatformStatsResponse {
    pub total_organizations: u64,
    pub total_products: u64,
    pub total_orders: u64,
    pub total_users: u64,
    pub database_backend: String,
    pub server_version: &'static str,
    pub superadmin_email: String,
    pub google_auth_enabled: bool,
    pub supported_modules_count: usize,
}

/// Retrieve platform-wide system and operational statistics for the Super Admin.
#[utoipa::path(
    get,
    path = "/api/v1/superadmin/stats",
    tag = "SuperAdmin",
    responses(
        (status = 200, description = "Platform statistics", body = PlatformStatsResponse)
    )
)]
pub async fn get_platform_stats(
    user: AuthUser,
    State(state): State<SharedState>,
) -> Result<Json<PlatformStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    user.require_superadmin().map_err(|(sc, msg)| {
        (
            sc,
            Json(ErrorResponse {
                error: msg.error.clone(),
            }),
        )
    })?;

    let total_organizations = organization::Entity::find()
        .count(state.reader())
        .await
        .unwrap_or(0);

    let total_products = product::Entity::find()
        .count(state.reader())
        .await
        .unwrap_or(0);

    let total_orders = order::Entity::find()
        .count(state.reader())
        .await
        .unwrap_or(0);

    let total_users = user::Entity::find()
        .count(state.reader())
        .await
        .unwrap_or(0);

    let backend = match state.db.get_database_backend() {
        DbBackend::Sqlite => "SQLite (WAL Mode)",
        DbBackend::Postgres => "PostgreSQL",
        DbBackend::MySql => "MySQL",
    };

    Ok(Json(PlatformStatsResponse {
        total_organizations,
        total_products,
        total_orders,
        total_users,
        database_backend: backend.to_string(),
        server_version: state.version,
        superadmin_email: state.superadmin.email.clone(),
        google_auth_enabled: state.superadmin.is_google_auth_enabled(),
        supported_modules_count: ALL_MODULES.len(),
    }))
}
