use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use obaf_database::entities::pos_shift;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct OpenShiftRequest {
    pub register_name: String,
    pub opening_float: Decimal,
}

#[derive(Deserialize, ToSchema)]
pub struct CloseShiftRequest {
    pub counted_cash: Decimal,
    pub notes: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct PosShiftResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub cashier_id: Uuid,
    pub register_name: String,
    pub status: String,
    pub opening_float: Decimal,
    pub cash_sales: Decimal,
    pub card_sales: Decimal,
    pub upi_sales: Decimal,
    pub cash_refunds: Decimal,
    pub expected_cash: Decimal,
    pub counted_cash: Option<Decimal>,
    pub variance: Option<Decimal>,
    pub notes: Option<String>,
    pub opened_at: chrono::DateTime<Utc>,
    pub closed_at: Option<chrono::DateTime<Utc>>,
}

impl From<pos_shift::Model> for PosShiftResponse {
    fn from(m: pos_shift::Model) -> Self {
        Self {
            id: m.id,
            tenant_id: m.tenant_id,
            cashier_id: m.cashier_id,
            register_name: m.register_name,
            status: m.status,
            opening_float: m.opening_float,
            cash_sales: m.cash_sales,
            card_sales: m.card_sales,
            upi_sales: m.upi_sales,
            cash_refunds: m.cash_refunds,
            expected_cash: m.expected_cash,
            counted_cash: m.counted_cash,
            variance: m.variance,
            notes: m.notes,
            opened_at: m.opened_at,
            closed_at: m.closed_at,
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/pos/shifts/open",
    tag = "POS",
    security(("bearer_auth" = [])),
    request_body = OpenShiftRequest,
    responses(
        (status = 201, description = "Cash drawer shift opened", body = PosShiftResponse),
        (status = 400, description = "Shift already open", body = AuthErrorResponse)
    )
)]
pub async fn open_shift(
    State(state): State<SharedState>,
    auth: AuthUser,
    Json(payload): Json<OpenShiftRequest>,
) -> Result<(StatusCode, Json<PosShiftResponse>), (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager", "staff", "cashier"])?;

    let tenant_uuid = auth.tenant_id.as_uuid();

    // Check if cashier already has an active open shift on this register
    let existing = pos_shift::Entity::find()
        .filter(pos_shift::Column::TenantId.eq(tenant_uuid))
        .filter(pos_shift::Column::CashierId.eq(auth.user_id))
        .filter(pos_shift::Column::Status.eq("OPEN"))
        .one(&state.db)
        .await
        .map_err(|e| internal_err(format!("DB error: {}", e)))?;

    if existing.is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error:
                    "Cashier already has an active open shift. Close it before opening a new shift."
                        .to_string(),
            }),
        ));
    }

    let shift_id = Uuid::new_v4();
    let now = Utc::now();

    let new_shift = pos_shift::ActiveModel {
        id: Set(shift_id),
        tenant_id: Set(tenant_uuid),
        cashier_id: Set(auth.user_id),
        register_name: Set(payload.register_name),
        status: Set("OPEN".to_string()),
        opening_float: Set(payload.opening_float),
        cash_sales: Set(Decimal::ZERO),
        card_sales: Set(Decimal::ZERO),
        upi_sales: Set(Decimal::ZERO),
        cash_refunds: Set(Decimal::ZERO),
        expected_cash: Set(payload.opening_float),
        counted_cash: Set(None),
        variance: Set(None),
        notes: Set(None),
        opened_at: Set(now),
        closed_at: Set(None),
    };

    let inserted = new_shift
        .insert(&state.db)
        .await
        .map_err(|e| internal_err(format!("Failed to open shift: {}", e)))?;

    state.broadcast_tenant_event(
        auth.tenant_id.as_uuid(),
        "pos.shift_opened",
        serde_json::json!({
            "shift_id": shift_id,
            "cashier_id": auth.user_id,
            "opening_float": payload.opening_float
        }),
    );

    Ok((StatusCode::CREATED, Json(inserted.into())))
}

#[utoipa::path(
    get,
    path = "/api/v1/pos/shifts/current",
    tag = "POS",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Active shift details", body = PosShiftResponse),
        (status = 404, description = "No open shift found", body = AuthErrorResponse)
    )
)]
pub async fn get_current_shift(
    State(state): State<SharedState>,
    auth: AuthUser,
) -> Result<Json<PosShiftResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let tenant_uuid = auth.tenant_id.as_uuid();

    let shift = pos_shift::Entity::find()
        .filter(pos_shift::Column::TenantId.eq(tenant_uuid))
        .filter(pos_shift::Column::CashierId.eq(auth.user_id))
        .filter(pos_shift::Column::Status.eq("OPEN"))
        .one(&state.db)
        .await
        .map_err(|e| internal_err(format!("DB error: {}", e)))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: "No active shift open for current user".to_string(),
                }),
            )
        })?;

    Ok(Json(shift.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/pos/shifts/{id}/close",
    tag = "POS",
    security(("bearer_auth" = [])),
    request_body = CloseShiftRequest,
    responses(
        (status = 200, description = "Shift closed and cash drawer reconciled", body = PosShiftResponse),
        (status = 400, description = "Shift already closed", body = AuthErrorResponse),
        (status = 404, description = "Shift not found", body = AuthErrorResponse)
    )
)]
pub async fn close_shift(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<CloseShiftRequest>,
) -> Result<Json<PosShiftResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    auth.require_role(&["admin", "manager", "staff", "cashier"])?;

    let tenant_uuid = auth.tenant_id.as_uuid();

    let shift = pos_shift::Entity::find()
        .filter(pos_shift::Column::Id.eq(id))
        .filter(pos_shift::Column::TenantId.eq(tenant_uuid))
        .one(&state.db)
        .await
        .map_err(|e| internal_err(format!("DB error: {}", e)))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: "Shift not found".to_string(),
                }),
            )
        })?;

    if shift.cashier_id != auth.user_id && auth.role != "admin" && auth.role != "manager" {
        return Err((
            StatusCode::FORBIDDEN,
            Json(AuthErrorResponse {
                error: "Cannot close another user's shift without administrative privileges"
                    .to_string(),
            }),
        ));
    }

    if shift.status == "CLOSED" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: "Shift is already closed".to_string(),
            }),
        ));
    }

    let now = Utc::now();
    let expected = shift.opening_float + shift.cash_sales - shift.cash_refunds;
    let variance = payload.counted_cash - expected;

    let mut active: pos_shift::ActiveModel = shift.into();
    active.status = Set("CLOSED".to_string());
    active.expected_cash = Set(expected);
    active.counted_cash = Set(Some(payload.counted_cash));
    active.variance = Set(Some(variance));
    active.notes = Set(payload.notes);
    active.closed_at = Set(Some(now));

    let updated = active
        .update(&state.db)
        .await
        .map_err(|e| internal_err(format!("Failed to close shift: {}", e)))?;

    state.broadcast_tenant_event(
        auth.tenant_id.as_uuid(),
        "pos.shift_closed",
        serde_json::json!({
            "shift_id": id,
            "expected_cash": expected,
            "counted_cash": payload.counted_cash,
            "variance": variance
        }),
    );

    Ok(Json(updated.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/pos/shifts",
    tag = "POS",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of POS shifts", body = Vec<PosShiftResponse>)
    )
)]
pub async fn list_shifts(
    State(state): State<SharedState>,
    auth: AuthUser,
) -> Result<Json<Vec<PosShiftResponse>>, (StatusCode, Json<AuthErrorResponse>)> {
    let shifts = pos_shift::Entity::find()
        .filter(pos_shift::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .order_by_desc(pos_shift::Column::OpenedAt)
        .limit(100)
        .all(&state.db)
        .await
        .map_err(|e| internal_err(format!("DB error: {}", e)))?;

    Ok(Json(shifts.into_iter().map(Into::into).collect()))
}

fn internal_err(msg: String) -> (StatusCode, Json<AuthErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(AuthErrorResponse { error: msg }),
    )
}
