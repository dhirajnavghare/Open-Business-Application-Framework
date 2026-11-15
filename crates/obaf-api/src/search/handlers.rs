use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use obaf_database::entities::{customer, order, product, supplier};
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Deserialize, IntoParams)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<u64>,
}

#[derive(Serialize, ToSchema)]
pub struct ProductSearchResult {
    pub id: Uuid,
    pub sku: String,
    pub name: String,
    pub price: Decimal,
}

#[derive(Serialize, ToSchema)]
pub struct CustomerSearchResult {
    pub id: Uuid,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct OrderSearchResult {
    pub id: Uuid,
    pub order_number: String,
    pub customer_name: Option<String>,
    pub grand_total: Decimal,
    pub status: String,
}

#[derive(Serialize, ToSchema)]
pub struct SupplierSearchResult {
    pub id: Uuid,
    pub name: String,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct GlobalSearchResponse {
    pub query: String,
    pub total_results: usize,
    pub products: Vec<ProductSearchResult>,
    pub customers: Vec<CustomerSearchResult>,
    pub orders: Vec<OrderSearchResult>,
    pub suppliers: Vec<SupplierSearchResult>,
}

#[utoipa::path(
    get,
    path = "/api/v1/search",
    tag = "Search",
    security(("bearer_auth" = [])),
    params(SearchQuery),
    responses(
        (status = 200, description = "Unified search results across catalog, customers, orders, and suppliers", body = GlobalSearchResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn global_search(
    auth: AuthUser,
    State(state): State<SharedState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<GlobalSearchResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let limit = params.limit.unwrap_or(10).clamp(1, 50);
    let term = params.q.trim();
    if term.is_empty() {
        return Ok(Json(GlobalSearchResponse {
            query: "".into(),
            total_results: 0,
            products: vec![],
            customers: vec![],
            orders: vec![],
            suppliers: vec![],
        }));
    }

    let pattern = format!("%{}%", term);

    // 1. Search Products
    let products = product::Entity::find()
        .filter(product::Column::TenantId.eq(auth.tenant_id.0))
        .filter(
            product::Column::Name
                .like(&pattern)
                .or(product::Column::Sku.like(&pattern)),
        )
        .limit(limit)
        .all(&state.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|p| ProductSearchResult {
            id: p.id,
            sku: p.sku,
            name: p.name,
            price: p.price,
        })
        .collect::<Vec<_>>();

    // 2. Search Customers
    let customers = customer::Entity::find()
        .filter(customer::Column::TenantId.eq(auth.tenant_id.0))
        .filter(
            customer::Column::Name
                .like(&pattern)
                .or(customer::Column::Phone.like(&pattern))
                .or(customer::Column::Email.like(&pattern)),
        )
        .limit(limit)
        .all(&state.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|c| CustomerSearchResult {
            id: c.id,
            name: c.name,
            phone: c.phone,
            email: c.email,
        })
        .collect::<Vec<_>>();

    // 3. Search Orders
    let orders = order::Entity::find()
        .filter(order::Column::TenantId.eq(auth.tenant_id.0))
        .filter(
            order::Column::OrderNumber
                .like(&pattern)
                .or(order::Column::CustomerName.like(&pattern)),
        )
        .limit(limit)
        .all(&state.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|o| OrderSearchResult {
            id: o.id,
            order_number: o.order_number,
            customer_name: o.customer_name,
            grand_total: o.grand_total,
            status: o.status,
        })
        .collect::<Vec<_>>();

    // 4. Search Suppliers
    let suppliers = supplier::Entity::find()
        .filter(supplier::Column::TenantId.eq(auth.tenant_id.0))
        .filter(
            supplier::Column::Name
                .like(&pattern)
                .or(supplier::Column::ContactPerson.like(&pattern)),
        )
        .limit(limit)
        .all(&state.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|s| SupplierSearchResult {
            id: s.id,
            name: s.name,
            contact_person: s.contact_person,
            phone: s.phone,
        })
        .collect::<Vec<_>>();

    let total_results = products.len() + customers.len() + orders.len() + suppliers.len();

    Ok(Json(GlobalSearchResponse {
        query: term.to_string(),
        total_results,
        products,
        customers,
        orders,
        suppliers,
    }))
}
