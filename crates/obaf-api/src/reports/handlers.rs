use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use obaf_database::entities::{
    booking, customer, order, order_item, payment, product, subscription,
};
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Serialize;
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Serialize, ToSchema)]
pub struct DashboardMetrics {
    pub total_revenue: Decimal,
    pub total_orders: usize,
    pub pending_orders: usize,
    pub completed_orders: usize,
    pub total_customers: usize,
    pub total_products: usize,
    pub total_bookings: usize,
    pub active_subscriptions: usize,
}

#[derive(Serialize, ToSchema)]
pub struct KitchenPrepItem {
    pub product_id: Uuid,
    pub item_name: String,
    pub pending_quantity: Decimal,
}

#[derive(Serialize, ToSchema)]
pub struct PaymentMethodSummary {
    pub method: String,
    pub count: usize,
    pub total_amount: Decimal,
}

#[utoipa::path(
    get,
    path = "/api/v1/reports/dashboard",
    tag = "Reports",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Tenant Executive KPIs", body = DashboardMetrics),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn get_dashboard_metrics(
    State(state): State<SharedState>,
    auth: AuthUser,
) -> Result<Json<DashboardMetrics>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager"])?;

    let tenant_uuid = auth.tenant_id.as_uuid();

    // 1. Total revenue from successful/completed payments
    let payments = payment::Entity::find()
        .filter(payment::Column::TenantId.eq(tenant_uuid))
        .filter(payment::Column::Status.is_in(["SUCCESS", "COMPLETED"]))
        .all(&state.db)
        .await
        .map_err(|e| internal_err(format!("DB Error: {}", e)))?;

    let total_revenue: Decimal = payments.iter().map(|p| p.amount).sum();

    // 2. Orders metrics
    let orders = order::Entity::find()
        .filter(order::Column::TenantId.eq(tenant_uuid))
        .all(&state.db)
        .await
        .map_err(|e| internal_err(format!("DB Error: {}", e)))?;

    let total_orders = orders.len();
    let pending_orders = orders
        .iter()
        .filter(|o| {
            let s = o.status.to_uppercase();
            s == "PENDING" || s == "ORDERED" || s == "COOKING" || s == "IN_PROGRESS"
        })
        .count();
    let completed_orders = orders
        .iter()
        .filter(|o| {
            let s = o.status.to_uppercase();
            s == "COMPLETED" || s == "SETTLED" || s == "SERVED"
        })
        .count();

    // 3. Customers count
    let total_customers = customer::Entity::find()
        .filter(customer::Column::TenantId.eq(tenant_uuid))
        .all(&state.db)
        .await
        .map_err(|e| internal_err(format!("DB Error: {}", e)))?
        .len();

    // 4. Products count
    let total_products = product::Entity::find()
        .filter(product::Column::TenantId.eq(tenant_uuid))
        .filter(product::Column::IsActive.eq(true))
        .all(&state.db)
        .await
        .map_err(|e| internal_err(format!("DB Error: {}", e)))?
        .len();

    // 5. Bookings count
    let total_bookings = booking::Entity::find()
        .filter(booking::Column::TenantId.eq(tenant_uuid))
        .all(&state.db)
        .await
        .map_err(|e| internal_err(format!("DB Error: {}", e)))?
        .len();

    // 6. Active subscriptions count
    let active_subscriptions = subscription::Entity::find()
        .filter(subscription::Column::TenantId.eq(tenant_uuid))
        .filter(subscription::Column::Status.eq("ACTIVE"))
        .all(&state.db)
        .await
        .map_err(|e| internal_err(format!("DB Error: {}", e)))?
        .len();

    Ok(Json(DashboardMetrics {
        total_revenue,
        total_orders,
        pending_orders,
        completed_orders,
        total_customers,
        total_products,
        total_bookings,
        active_subscriptions,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/reports/kitchen-prep",
    tag = "Reports",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Aggregated kitchen prep item list", body = Vec<KitchenPrepItem>),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn get_kitchen_prep_sheet(
    State(state): State<SharedState>,
    auth: AuthUser,
) -> Result<Json<Vec<KitchenPrepItem>>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager", "staff", "cashier", "waiter"])?;

    let tenant_uuid = auth.tenant_id.as_uuid();

    // Fetch orders in active prep stages
    let active_orders = order::Entity::find()
        .filter(order::Column::TenantId.eq(tenant_uuid))
        .all(&state.db)
        .await
        .map_err(|e| internal_err(format!("DB Error: {}", e)))?;

    let prep_order_ids: Vec<uuid::Uuid> = active_orders
        .into_iter()
        .filter(|o| {
            let s = o.status.to_uppercase();
            s == "PENDING"
                || s == "ORDERED"
                || s == "CONFIRMED"
                || s == "COOKING"
                || s == "IN_PROGRESS"
        })
        .map(|o| o.id)
        .collect();

    if prep_order_ids.is_empty() {
        return Ok(Json(Vec::new()));
    }

    let items = order_item::Entity::find()
        .filter(order_item::Column::OrderId.is_in(prep_order_ids))
        .all(&state.db)
        .await
        .map_err(|e| internal_err(format!("DB Error: {}", e)))?;

    let mut map: HashMap<(Uuid, String), Decimal> = HashMap::new();
    for it in items {
        *map.entry((it.product_id, it.item_name))
            .or_insert(Decimal::ZERO) += it.quantity;
    }

    let result: Vec<KitchenPrepItem> = map
        .into_iter()
        .map(
            |((product_id, item_name), pending_quantity)| KitchenPrepItem {
                product_id,
                item_name,
                pending_quantity,
            },
        )
        .collect();

    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/api/v1/reports/sales-by-payment-method",
    tag = "Reports",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Revenue breakdown by payment tender", body = Vec<PaymentMethodSummary>),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn get_sales_by_payment_method(
    State(state): State<SharedState>,
    auth: AuthUser,
) -> Result<Json<Vec<PaymentMethodSummary>>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager"])?;

    let tenant_uuid = auth.tenant_id.as_uuid();

    let payments = payment::Entity::find()
        .filter(payment::Column::TenantId.eq(tenant_uuid))
        .filter(payment::Column::Status.is_in(["SUCCESS", "COMPLETED"]))
        .all(&state.db)
        .await
        .map_err(|e| internal_err(format!("DB Error: {}", e)))?;

    let mut map: HashMap<String, (usize, Decimal)> = HashMap::new();
    for p in payments {
        let entry = map.entry(p.payment_method).or_insert((0, Decimal::ZERO));
        entry.0 += 1;
        entry.1 += p.amount;
    }

    let result: Vec<PaymentMethodSummary> = map
        .into_iter()
        .map(|(method, (count, total_amount))| PaymentMethodSummary {
            method,
            count,
            total_amount,
        })
        .collect();

    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/api/v1/reports/export/orders.csv",
    tag = "Reports",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "CSV export of orders", content_type = "text/csv")
    )
)]
pub async fn export_orders_csv(
    State(state): State<SharedState>,
    auth: AuthUser,
) -> Result<Response, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager"])?;

    let orders = order::Entity::find()
        .filter(order::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .all(&state.db)
        .await
        .map_err(|e| internal_err(format!("DB Error: {}", e)))?;

    let mut csv = String::from(
        "id,order_number,customer_name,table_or_room,status,subtotal,tax_total,grand_total,created_at\n",
    );
    for o in orders {
        let grand = o.subtotal + o.tax_total - o.discount_total;
        csv.push_str(&format!(
            "{},\"{}\",\"{}\",\"{}\",{},{},{},{},{}\n",
            o.id,
            o.order_number,
            o.customer_name.unwrap_or_default().replace('"', "\"\""),
            o.table_or_room.unwrap_or_default().replace('"', "\"\""),
            o.status,
            o.subtotal,
            o.tax_total,
            grand,
            o.created_at.to_rfc3339()
        ));
    }

    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"orders_export.csv\"",
            ),
        ],
        csv,
    )
        .into_response())
}

#[utoipa::path(
    get,
    path = "/api/v1/reports/export/customers.csv",
    tag = "Reports",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "CSV export of customers", content_type = "text/csv")
    )
)]
pub async fn export_customers_csv(
    State(state): State<SharedState>,
    auth: AuthUser,
) -> Result<Response, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager"])?;

    let customers = customer::Entity::find()
        .filter(customer::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .all(&state.db)
        .await
        .map_err(|e| internal_err(format!("DB Error: {}", e)))?;

    let mut csv = String::from("id,name,email,phone,tax_id,address,created_at\n");
    for c in customers {
        csv.push_str(&format!(
            "{},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",{}\n",
            c.id,
            c.name.replace('"', "\"\""),
            c.email.unwrap_or_default().replace('"', "\"\""),
            c.phone.unwrap_or_default().replace('"', "\"\""),
            c.tax_id.unwrap_or_default().replace('"', "\"\""),
            c.address.unwrap_or_default().replace('"', "\"\""),
            c.created_at.to_rfc3339()
        ));
    }

    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"customers_export.csv\"",
            ),
        ],
        csv,
    )
        .into_response())
}

#[utoipa::path(
    get,
    path = "/api/v1/reports/export/inventory.csv",
    tag = "Reports",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "CSV export of catalog and products", content_type = "text/csv")
    )
)]
pub async fn export_inventory_csv(
    State(state): State<SharedState>,
    auth: AuthUser,
) -> Result<Response, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager"])?;

    let products = product::Entity::find()
        .filter(product::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .all(&state.db)
        .await
        .map_err(|e| internal_err(format!("DB Error: {}", e)))?;

    let mut csv = String::from("id,sku,name,price,cost_price,tax_rate,is_active,created_at\n");
    for p in products {
        csv.push_str(&format!(
            "{},\"{}\",\"{}\",{},{},{},{},{}\n",
            p.id,
            p.sku.replace('"', "\"\""),
            p.name.replace('"', "\"\""),
            p.price,
            p.cost_price,
            p.tax_rate,
            p.is_active,
            p.created_at.to_rfc3339()
        ));
    }

    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"inventory_export.csv\"",
            ),
        ],
        csv,
    )
        .into_response())
}

fn internal_err(msg: String) -> (StatusCode, Json<AuthErrorResponse>) {
    tracing::error!("Internal error in reports: {}", msg);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(AuthErrorResponse {
            error: "A database error occurred".to_string(),
        }),
    )
}
