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

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use obaf_core::security::hash_password;
use obaf_database::entities::{order, organization, product, user};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, ModelTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::extractor::AuthUser;
use crate::state::SharedState;

/// Catalog descriptor for an individual OBAF API module
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ModuleCatalogItem {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub default_enabled: bool,
}

pub const ALL_MODULES: &[ModuleCatalogItem] = &[
    ModuleCatalogItem {
        id: "catalog",
        name: "Catalog & Products",
        description: "Products, categories, brands, variants, SKU management & barcodes",
        category: "Core Operations",
        default_enabled: true,
    },
    ModuleCatalogItem {
        id: "inventory",
        name: "Inventory & Stock",
        description: "Multi-warehouse tracking, stock transfers, batches & stock ledger",
        category: "Core Operations",
        default_enabled: true,
    },
    ModuleCatalogItem {
        id: "orders",
        name: "Orders & Sales",
        description: "Order lifecycle, status workflows, line items & order fulfillment",
        category: "Core Operations",
        default_enabled: true,
    },
    ModuleCatalogItem {
        id: "billing",
        name: "Billing & Invoicing",
        description: "Tax invoices, credit notes, payment schedules & PDF document generation",
        category: "Finance & Revenue",
        default_enabled: true,
    },
    ModuleCatalogItem {
        id: "payments",
        name: "Payments & Settlements",
        description: "Payment capture, gateway integrations, refunds & settlement ledger",
        category: "Finance & Revenue",
        default_enabled: true,
    },
    ModuleCatalogItem {
        id: "subscriptions",
        name: "Subscriptions & Recurring",
        description: "Recurring subscription plans, billing cycles & automatic renewals",
        category: "Finance & Revenue",
        default_enabled: false,
    },
    ModuleCatalogItem {
        id: "customers",
        name: "Customers & CRM",
        description: "Customer accounts, profiles, loyalty points & purchase history",
        category: "Sales & CRM",
        default_enabled: true,
    },
    ModuleCatalogItem {
        id: "promotions",
        name: "Promotions & Discounts",
        description: "Coupon codes, discount rules, buy-X-get-Y & seasonal campaigns",
        category: "Sales & CRM",
        default_enabled: false,
    },
    ModuleCatalogItem {
        id: "suppliers",
        name: "Suppliers & Vendors",
        description: "Vendor directory, contact management & catalog price agreements",
        category: "Supply Chain",
        default_enabled: true,
    },
    ModuleCatalogItem {
        id: "procurement",
        name: "Procurement & POs",
        description: "Purchase orders, goods receipt notes (GRN) & supplier 3-way matching",
        category: "Supply Chain",
        default_enabled: true,
    },
    ModuleCatalogItem {
        id: "pos",
        name: "Point of Sale (POS)",
        description: "In-store cash register, barcode scanner, receipt printing & quick checkout",
        category: "Specialized Engines",
        default_enabled: false,
    },
    ModuleCatalogItem {
        id: "bookings",
        name: "Bookings & Appointments",
        description: "Appointment scheduling, service bookings, staff calendars & capacity",
        category: "Specialized Engines",
        default_enabled: false,
    },
    ModuleCatalogItem {
        id: "reports",
        name: "Reports & Analytics",
        description: "Financial performance, sales analytics, and inventory valuation",
        category: "Specialized Engines",
        default_enabled: true,
    },
    ModuleCatalogItem {
        id: "webhooks",
        name: "Webhooks & Automation",
        description: "Real-time HTTP event dispatch, delivery logs & retry queues",
        category: "Platform & Integration",
        default_enabled: true,
    },
    ModuleCatalogItem {
        id: "sync",
        name: "Offline Sync Engine",
        description: "Vector clock multi-master synchronization for offline POS and edge nodes",
        category: "Platform & Integration",
        default_enabled: false,
    },
];

#[derive(Debug, Serialize, ToSchema)]
pub struct OrganizationSummary {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub preset: String,
    pub active_modules: Vec<String>,
    pub settings: serde_json::Value,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub products_count: u64,
    pub orders_count: u64,
    pub users_count: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserSummary {
    pub id: Uuid,
    pub email: String,
    pub full_name: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrganizationDetail {
    pub organization: OrganizationSummary,
    pub users: Vec<UserSummary>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub slug: String,
    pub preset: Option<String>,
    /// Explicit list of enabled API module IDs for this organization.
    /// If omitted, defaults based on preset or standard core modules.
    pub active_modules: Option<Vec<String>>,
    pub admin_name: String,
    pub admin_email: String,
    pub admin_password: Option<String>,
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateOrganizationRequest {
    pub name: Option<String>,
    /// Dynamically update which API modules are enabled/supported for this organization.
    pub active_modules: Option<Vec<String>>,
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

/// Retrieve the full catalog of supported API modules.
#[utoipa::path(
    get,
    path = "/api/v1/superadmin/modules",
    tag = "SuperAdmin",
    responses(
        (status = 200, description = "List of all supported modules", body = Vec<ModuleCatalogItem>)
    )
)]
pub async fn list_supported_modules(
    user: AuthUser,
) -> Result<Json<Vec<ModuleCatalogItem>>, (StatusCode, Json<ErrorResponse>)> {
    user.require_superadmin().map_err(|(sc, msg)| {
        (
            sc,
            Json(ErrorResponse {
                error: msg.error.clone(),
            }),
        )
    })?;

    Ok(Json(ALL_MODULES.to_vec()))
}

/// List all organizations with usage statistics and active modules.
#[utoipa::path(
    get,
    path = "/api/v1/superadmin/organizations",
    tag = "SuperAdmin",
    responses(
        (status = 200, description = "List of all organizations", body = Vec<OrganizationSummary>)
    )
)]
pub async fn list_organizations(
    user: AuthUser,
    State(state): State<SharedState>,
) -> Result<Json<Vec<OrganizationSummary>>, (StatusCode, Json<ErrorResponse>)> {
    user.require_superadmin().map_err(|(sc, msg)| {
        (
            sc,
            Json(ErrorResponse {
                error: msg.error.clone(),
            }),
        )
    })?;

    let orgs = organization::Entity::find()
        .order_by_desc(organization::Column::CreatedAt)
        .all(state.reader())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to query organizations: {}", e),
                }),
            )
        })?;

    let mut summaries = Vec::with_capacity(orgs.len());

    for org in orgs {
        let active_modules: Vec<String> =
            serde_json::from_str(&org.active_modules).unwrap_or_default();
        let settings: serde_json::Value =
            serde_json::from_str(&org.settings).unwrap_or(serde_json::json!({}));

        let products_count = product::Entity::find()
            .filter(product::Column::TenantId.eq(org.id))
            .count(state.reader())
            .await
            .unwrap_or(0);

        let orders_count = order::Entity::find()
            .filter(order::Column::TenantId.eq(org.id))
            .count(state.reader())
            .await
            .unwrap_or(0);

        let users_count = user::Entity::find()
            .filter(user::Column::OrganizationId.eq(org.id))
            .count(state.reader())
            .await
            .unwrap_or(0);

        summaries.push(OrganizationSummary {
            id: org.id,
            name: org.name,
            slug: org.slug,
            preset: org.preset,
            active_modules,
            settings,
            created_at: org.created_at,
            updated_at: org.updated_at,
            products_count,
            orders_count,
            users_count,
        });
    }

    Ok(Json(summaries))
}

/// Retrieve specific organization details including users and active modules.
#[utoipa::path(
    get,
    path = "/api/v1/superadmin/organizations/:id",
    tag = "SuperAdmin",
    responses(
        (status = 200, description = "Organization detail", body = OrganizationDetail),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn get_organization(
    user: AuthUser,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Result<Json<OrganizationDetail>, (StatusCode, Json<ErrorResponse>)> {
    user.require_superadmin().map_err(|(sc, msg)| {
        (
            sc,
            Json(ErrorResponse {
                error: msg.error.clone(),
            }),
        )
    })?;

    let org = organization::Entity::find_by_id(id)
        .one(state.reader())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Organization with id '{}' not found", id),
                }),
            )
        })?;

    let users = user::Entity::find()
        .filter(user::Column::OrganizationId.eq(id))
        .all(state.reader())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?;

    let products_count = product::Entity::find()
        .filter(product::Column::TenantId.eq(id))
        .count(state.reader())
        .await
        .unwrap_or(0);

    let orders_count = order::Entity::find()
        .filter(order::Column::TenantId.eq(id))
        .count(state.reader())
        .await
        .unwrap_or(0);

    let active_modules: Vec<String> = serde_json::from_str(&org.active_modules).unwrap_or_default();
    let settings: serde_json::Value =
        serde_json::from_str(&org.settings).unwrap_or(serde_json::json!({}));

    let user_summaries = users
        .into_iter()
        .map(|u| UserSummary {
            id: u.id,
            email: u.email,
            full_name: u.full_name,
            role: u.role,
            is_active: u.is_active,
            created_at: u.created_at,
        })
        .collect::<Vec<_>>();

    let org_summary = OrganizationSummary {
        id: org.id,
        name: org.name,
        slug: org.slug,
        preset: org.preset,
        active_modules,
        settings,
        created_at: org.created_at,
        updated_at: org.updated_at,
        products_count,
        orders_count,
        users_count: user_summaries.len() as u64,
    };

    Ok(Json(OrganizationDetail {
        organization: org_summary,
        users: user_summaries,
    }))
}

/// Create a new organization with custom supported API modules and initial admin user.
#[utoipa::path(
    post,
    path = "/api/v1/superadmin/organizations",
    tag = "SuperAdmin",
    request_body = CreateOrganizationRequest,
    responses(
        (status = 201, description = "Organization created successfully", body = OrganizationSummary),
        (status = 409, description = "Slug or admin email already exists", body = ErrorResponse)
    )
)]
pub async fn create_organization(
    user: AuthUser,
    State(state): State<SharedState>,
    Json(payload): Json<CreateOrganizationRequest>,
) -> Result<(StatusCode, Json<OrganizationSummary>), (StatusCode, Json<ErrorResponse>)> {
    user.require_superadmin().map_err(|(sc, msg)| {
        (
            sc,
            Json(ErrorResponse {
                error: msg.error.clone(),
            }),
        )
    })?;

    let slug_clean = payload.slug.trim().to_lowercase();
    if slug_clean.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Organization slug cannot be empty.".to_string(),
            }),
        ));
    }

    // Check slug uniqueness
    let existing = organization::Entity::find()
        .filter(organization::Column::Slug.eq(&slug_clean))
        .one(state.reader())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?;

    if existing.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: format!("Organization with slug '{}' already exists.", slug_clean),
            }),
        ));
    }

    // Determine enabled modules
    let active_modules = payload.active_modules.unwrap_or_else(|| {
        ALL_MODULES
            .iter()
            .filter(|m| m.default_enabled)
            .map(|m| m.id.to_string())
            .collect()
    });

    let modules_json = serde_json::to_string(&active_modules).unwrap_or_else(|_| "[]".to_string());
    let settings_json = payload
        .settings
        .map(|s| serde_json::to_string(&s).unwrap_or_else(|_| "{}".to_string()))
        .unwrap_or_else(|| "{}".to_string());

    let org_id = Uuid::new_v4();
    let now = Utc::now();
    let preset = payload
        .preset
        .unwrap_or_else(|| "retail_grocery".to_string());

    // Insert organization into primary DB
    let new_org = organization::ActiveModel {
        id: Set(org_id),
        name: Set(payload.name.trim().to_string()),
        slug: Set(slug_clean.clone()),
        preset: Set(preset.clone()),
        active_modules: Set(modules_json.clone()),
        settings: Set(settings_json.clone()),
        created_at: Set(now),
        updated_at: Set(now),
    };

    new_org.insert(state.writer()).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to create organization: {}", e),
            }),
        )
    })?;

    // Create initial Tenant Admin user
    let user_id = Uuid::new_v4();
    let raw_pw = payload
        .admin_password
        .unwrap_or_else(|| "admin123".to_string());
    let pw_hash = hash_password(&raw_pw).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to hash password: {}", e),
            }),
        )
    })?;

    let admin_user = user::ActiveModel {
        id: Set(user_id),
        organization_id: Set(org_id),
        email: Set(payload.admin_email.trim().to_lowercase()),
        password_hash: Set(pw_hash),
        full_name: Set(payload.admin_name.trim().to_string()),
        role: Set("admin".to_string()),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    };

    admin_user.insert(state.writer()).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to create admin user: {}", e),
            }),
        )
    })?;

    // Pre-warm module cache for the new tenant
    state
        .cache
        .insert(format!("tenant_modules:{}", org_id), modules_json);

    let summary = OrganizationSummary {
        id: org_id,
        name: payload.name.trim().to_string(),
        slug: slug_clean,
        preset,
        active_modules,
        settings: serde_json::from_str(&settings_json).unwrap_or(serde_json::json!({})),
        created_at: now,
        updated_at: now,
        products_count: 0,
        orders_count: 0,
        users_count: 1,
    };

    Ok((StatusCode::CREATED, Json(summary)))
}

/// Update an organization's configuration and dynamically toggle its supported API modules.
#[utoipa::path(
    put,
    path = "/api/v1/superadmin/organizations/:id",
    tag = "SuperAdmin",
    request_body = UpdateOrganizationRequest,
    responses(
        (status = 200, description = "Organization updated successfully", body = OrganizationSummary),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn update_organization(
    user: AuthUser,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateOrganizationRequest>,
) -> Result<Json<OrganizationSummary>, (StatusCode, Json<ErrorResponse>)> {
    user.require_superadmin().map_err(|(sc, msg)| {
        (
            sc,
            Json(ErrorResponse {
                error: msg.error.clone(),
            }),
        )
    })?;

    let org = organization::Entity::find_by_id(id)
        .one(state.reader())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Organization with id '{}' not found", id),
                }),
            )
        })?;

    let mut active_model: organization::ActiveModel = org.clone().into();
    let now = Utc::now();
    active_model.updated_at = Set(now);

    if let Some(new_name) = payload.name {
        let name_clean = new_name.trim().to_string();
        if !name_clean.is_empty() {
            active_model.name = Set(name_clean);
        }
    }

    let final_modules = if let Some(new_modules) = payload.active_modules {
        let json_str = serde_json::to_string(&new_modules).unwrap_or_else(|_| "[]".to_string());
        active_model.active_modules = Set(json_str.clone());

        // Update in-memory TTL cache immediately for seamless zero-downtime gate change
        state
            .cache
            .insert(format!("tenant_modules:{}", id), json_str);
        new_modules
    } else {
        serde_json::from_str(&org.active_modules).unwrap_or_default()
    };

    let final_settings = if let Some(new_settings) = payload.settings {
        let json_str = serde_json::to_string(&new_settings).unwrap_or_else(|_| "{}".to_string());
        active_model.settings = Set(json_str);
        new_settings
    } else {
        serde_json::from_str(&org.settings).unwrap_or(serde_json::json!({}))
    };

    let updated_org = active_model.update(state.writer()).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to update organization: {}", e),
            }),
        )
    })?;

    let products_count = product::Entity::find()
        .filter(product::Column::TenantId.eq(id))
        .count(state.reader())
        .await
        .unwrap_or(0);

    let orders_count = order::Entity::find()
        .filter(order::Column::TenantId.eq(id))
        .count(state.reader())
        .await
        .unwrap_or(0);

    let users_count = user::Entity::find()
        .filter(user::Column::OrganizationId.eq(id))
        .count(state.reader())
        .await
        .unwrap_or(0);

    Ok(Json(OrganizationSummary {
        id: updated_org.id,
        name: updated_org.name,
        slug: updated_org.slug,
        preset: updated_org.preset,
        active_modules: final_modules,
        settings: final_settings,
        created_at: updated_org.created_at,
        updated_at: updated_org.updated_at,
        products_count,
        orders_count,
        users_count,
    }))
}

/// Delete an organization and its associated records.
#[utoipa::path(
    delete,
    path = "/api/v1/superadmin/organizations/:id",
    tag = "SuperAdmin",
    responses(
        (status = 204, description = "Organization deleted successfully"),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn delete_organization(
    user: AuthUser,
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    user.require_superadmin().map_err(|(sc, msg)| {
        (
            sc,
            Json(ErrorResponse {
                error: msg.error.clone(),
            }),
        )
    })?;

    let org = organization::Entity::find_by_id(id)
        .one(state.reader())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Organization with id '{}' not found", id),
                }),
            )
        })?;

    // Invalidate module cache
    state.cache.remove(&format!("tenant_modules:{}", id));

    // Delete associated users first
    user::Entity::delete_many()
        .filter(user::Column::OrganizationId.eq(id))
        .exec(state.writer())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to delete organization users: {}", e),
                }),
            )
        })?;

    // Delete the organization
    org.delete(state.writer()).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to delete organization: {}", e),
            }),
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}
