use axum::{extract::State, http::StatusCode, Json};
use chrono::{DateTime, Utc};
use obaf_database::entities::payment;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
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
    Json(payload): Json<RecordPaymentRequest>,
) -> Result<(StatusCode, Json<PaymentResponse>), (StatusCode, Json<AuthErrorResponse>)> {
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

    Ok((StatusCode::CREATED, Json(inserted.into())))
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
