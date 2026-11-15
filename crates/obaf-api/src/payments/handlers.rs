use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use obaf_database::entities::payment;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct RecordPaymentRequest {
    pub order_id: Option<Uuid>,
    pub customer_id: Option<Uuid>,
    pub amount: Decimal,
    pub payment_method: String, // "CASH", "CARD", "UPI", "BANK_TRANSFER"
    pub reference_number: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct PaymentResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub order_id: Option<Uuid>,
    pub customer_id: Option<Uuid>,
    pub amount: Decimal,
    pub payment_method: String,
    pub reference_number: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct DailySettlementSummary {
    pub total_revenue: Decimal,
    pub cash_total: Decimal,
    pub card_total: Decimal,
    pub upi_total: Decimal,
    pub bank_transfer_total: Decimal,
    pub total_transactions: usize,
}

impl From<payment::Model> for PaymentResponse {
    fn from(m: payment::Model) -> Self {
        Self {
            id: m.id,
            tenant_id: m.tenant_id,
            order_id: m.order_id,
            customer_id: m.customer_id,
            amount: m.amount,
            payment_method: m.payment_method,
            reference_number: m.reference_number,
            status: m.status,
            created_at: m.created_at,
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/payments",
    tag = "Payments",
    security(("bearer_auth" = [])),
    request_body = RecordPaymentRequest,
    responses(
        (status = 201, description = "Payment recorded successfully", body = PaymentResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn record_payment(
    State(state): State<SharedState>,
    auth: AuthUser,
    headers: HeaderMap,
    Json(payload): Json<RecordPaymentRequest>,
) -> Result<axum::response::Response, (StatusCode, Json<AuthErrorResponse>)> {
    // Distributed Systems Pattern: Idempotency Key deduplication (System Design Primer)
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if let Some(ref ikey) = idempotency_key {
        let cache_key = format!("idempotency:payment:{}:{}", auth.tenant_id.0, ikey);
        if let Some((cached_status, cached_json)) = state.idempotency_cache.get(&cache_key) {
            let mut res_headers = HeaderMap::new();
            res_headers.insert(
                HeaderName::from_static("x-idempotent-replay"),
                HeaderValue::from_static("true"),
            );
            return Ok((cached_status, res_headers, Json(cached_json)).into_response());
        }
    }

    let payment_id = Uuid::new_v4();
    let now = Utc::now();

    let new_payment = payment::ActiveModel {
        id: Set(payment_id),
        tenant_id: Set(auth.tenant_id.as_uuid()),
        order_id: Set(payload.order_id),
        customer_id: Set(payload.customer_id),
        amount: Set(payload.amount),
        payment_method: Set(payload.payment_method.to_uppercase()),
        reference_number: Set(payload.reference_number),
        status: Set("SUCCESS".to_string()),
        created_at: Set(now),
    };

    let inserted = new_payment.insert(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to record payment: {}", e),
            }),
        )
    })?;

    let response: PaymentResponse = inserted.into();

    if let Some(ref ikey) = idempotency_key {
        let cache_key = format!("idempotency:payment:{}:{}", auth.tenant_id.0, ikey);
        if let Ok(val) = serde_json::to_value(&response) {
            state
                .idempotency_cache
                .insert(cache_key, (StatusCode::CREATED, val));
        }
    }

    Ok((StatusCode::CREATED, Json(response)).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v1/payments/settlement",
    tag = "Payments",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "End-of-day revenue & payment settlement summary", body = DailySettlementSummary),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn daily_settlement(
    State(state): State<SharedState>,
    auth: AuthUser,
) -> Result<Json<DailySettlementSummary>, (StatusCode, Json<AuthErrorResponse>)> {
    let payments = payment::Entity::find()
        .filter(payment::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .filter(payment::Column::Status.eq("SUCCESS"))
        .all(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?;

    let mut total_revenue = Decimal::ZERO;
    let mut cash_total = Decimal::ZERO;
    let mut card_total = Decimal::ZERO;
    let mut upi_total = Decimal::ZERO;
    let mut bank_transfer_total = Decimal::ZERO;
    let count = payments.len();

    for p in payments {
        total_revenue += p.amount;
        match p.payment_method.as_str() {
            "CASH" => cash_total += p.amount,
            "CARD" => card_total += p.amount,
            "UPI" => upi_total += p.amount,
            "BANK_TRANSFER" => bank_transfer_total += p.amount,
            _ => cash_total += p.amount,
        }
    }

    Ok(Json(DailySettlementSummary {
        total_revenue,
        cash_total,
        card_total,
        upi_total,
        bank_transfer_total,
        total_transactions: count,
    }))
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct PaymentFilterQuery {
    pub order_id: Option<Uuid>,
    pub customer_id: Option<Uuid>,
    pub payment_method: Option<String>,
    pub status: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Deserialize, ToSchema)]
pub struct RefundPaymentRequest {
    pub reason: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/payments",
    tag = "Payments",
    security(("bearer_auth" = [])),
    params(
        PaymentFilterQuery
    ),
    responses(
        (status = 200, description = "List of payments", body = Vec<PaymentResponse>),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn list_payments(
    State(state): State<SharedState>,
    auth: AuthUser,
    Query(query): Query<PaymentFilterQuery>,
) -> Result<Json<Vec<PaymentResponse>>, (StatusCode, Json<AuthErrorResponse>)> {
    let mut q = payment::Entity::find()
        .filter(payment::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .order_by_desc(payment::Column::CreatedAt);

    if let Some(oid) = query.order_id {
        q = q.filter(payment::Column::OrderId.eq(oid));
    }

    if let Some(cid) = query.customer_id {
        q = q.filter(payment::Column::CustomerId.eq(cid));
    }

    if let Some(pm) = query.payment_method {
        q = q.filter(payment::Column::PaymentMethod.eq(pm.to_uppercase()));
    }

    if let Some(st) = query.status {
        q = q.filter(payment::Column::Status.eq(st.to_uppercase()));
    }

    let limit = query.limit.unwrap_or(50).min(200);
    q = q.limit(limit);

    if let Some(offset) = query.offset {
        q = q.offset(offset);
    }

    let payments = q.all(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Database error: {}", e),
            }),
        )
    })?;

    let resp = payments.into_iter().map(Into::into).collect();
    Ok(Json(resp))
}

#[utoipa::path(
    post,
    path = "/api/v1/payments/{id}/refund",
    tag = "Payments",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Payment ID to refund")
    ),
    request_body = RefundPaymentRequest,
    responses(
        (status = 200, description = "Payment refunded successfully", body = PaymentResponse),
        (status = 400, description = "Payment already refunded or invalid", body = AuthErrorResponse),
        (status = 404, description = "Payment not found", body = AuthErrorResponse)
    )
)]
pub async fn refund_payment(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<RefundPaymentRequest>,
) -> Result<Json<PaymentResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager"])?;

    let payment_model = payment::Entity::find()
        .filter(payment::Column::Id.eq(id))
        .filter(payment::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Database error: {}", e),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: "Payment not found".to_string(),
                }),
            )
        })?;

    if payment_model.status.to_uppercase() == "REFUNDED" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: "Payment has already been refunded".to_string(),
            }),
        ));
    }

    let original_amount = payment_model.amount;
    let mut active: payment::ActiveModel = payment_model.into();
    active.status = Set("REFUNDED".to_string());

    let updated = active.update(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to update payment status: {}", e),
            }),
        )
    })?;

    // Record audit log
    crate::admin::audit::record_audit_log(
        &state.db,
        crate::admin::audit::NewAuditEntry {
            tenant_id: auth.tenant_id.as_uuid(),
            actor_id: Some(auth.user_id),
            action: "PAYMENT_REFUNDED".to_string(),
            entity_type: "payment".to_string(),
            entity_id: Some(id.to_string()),
            details: serde_json::json!({
                "amount": original_amount,
                "reason": payload.reason
            }),
            ip_address: None,
        },
    )
    .await;

    // Real-time broadcast
    state.broadcast_event(
        "payment.refunded",
        serde_json::json!({
            "payment_id": id,
            "amount": original_amount,
            "reason": payload.reason
        }),
    );

    Ok(Json(updated.into()))
}
