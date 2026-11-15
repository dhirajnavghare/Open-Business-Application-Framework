use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use obaf_database::entities::coupon;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct CreateCouponRequest {
    pub code: String,
    pub discount_type: String, // "PERCENTAGE", "FIXED_AMOUNT"
    pub discount_value: Decimal,
    pub min_order_amount: Option<Decimal>,
    pub max_discount_amount: Option<Decimal>,
    pub usage_limit: Option<i32>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub is_active: Option<bool>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CouponResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub code: String,
    pub discount_type: String,
    pub discount_value: Decimal,
    pub min_order_amount: Option<Decimal>,
    pub max_discount_amount: Option<Decimal>,
    pub usage_limit: Option<i32>,
    pub used_count: i32,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<coupon::Model> for CouponResponse {
    fn from(m: coupon::Model) -> Self {
        Self {
            id: m.id,
            tenant_id: m.tenant_id,
            code: m.code,
            discount_type: m.discount_type,
            discount_value: m.discount_value,
            min_order_amount: m.min_order_amount,
            max_discount_amount: m.max_discount_amount,
            usage_limit: m.usage_limit,
            used_count: m.used_count,
            valid_from: m.valid_from,
            valid_until: m.valid_until,
            is_active: m.is_active,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(Deserialize, IntoParams)]
pub struct ListCouponsQuery {
    pub active_only: Option<bool>,
}

#[derive(Deserialize, ToSchema)]
pub struct ValidateCouponRequest {
    pub code: String,
    pub subtotal: Decimal,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ValidateCouponResponse {
    pub is_valid: bool,
    pub code: String,
    pub discount_type: Option<String>,
    pub discount_value: Option<Decimal>,
    pub discount_amount: Decimal,
    pub final_subtotal: Decimal,
    pub message: String,
}

#[derive(Deserialize, ToSchema)]
pub struct RedeemCouponRequest {
    pub subtotal: Decimal,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RedeemCouponResponse {
    pub is_valid: bool,
    pub code: String,
    pub discount_amount: Decimal,
    pub final_subtotal: Decimal,
    pub used_count: i32,
    pub remaining_usage: Option<i32>,
}

fn calculate_discount(
    m: &coupon::Model,
    subtotal: Decimal,
    now: DateTime<Utc>,
) -> (bool, Decimal, String) {
    if !m.is_active {
        return (false, Decimal::ZERO, "Coupon is inactive".to_string());
    }

    if let Some(from) = m.valid_from {
        if now < from {
            return (false, Decimal::ZERO, "Coupon is not yet active".to_string());
        }
    }

    if let Some(until) = m.valid_until {
        if now > until {
            return (false, Decimal::ZERO, "Coupon has expired".to_string());
        }
    }

    if let Some(limit) = m.usage_limit {
        if m.used_count >= limit {
            return (
                false,
                Decimal::ZERO,
                "Coupon usage limit has been reached".to_string(),
            );
        }
    }

    if let Some(min_amt) = m.min_order_amount {
        if subtotal < min_amt {
            return (
                false,
                Decimal::ZERO,
                format!("Order subtotal must be at least {}", min_amt),
            );
        }
    }

    let discount = match m.discount_type.as_str() {
        "PERCENTAGE" => {
            let mut disc = (subtotal * m.discount_value).round_dp(2);
            if let Some(max_val) = m.max_discount_amount {
                disc = disc.min(max_val);
            }
            disc
        }
        "FIXED_AMOUNT" => m.discount_value.min(subtotal),
        _ => Decimal::ZERO,
    };

    (true, discount, "Coupon is valid".to_string())
}

#[utoipa::path(
    post,
    path = "/api/v1/promotions/coupons",
    tag = "Promotions",
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
    request_body = CreateCouponRequest,
    responses(
        (status = 201, description = "Coupon created successfully", body = CouponResponse),
        (status = 400, description = "Validation error", body = AuthErrorResponse)
    )
)]
pub async fn create_coupon(
    State(state): State<SharedState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateCouponRequest>,
) -> Result<(StatusCode, Json<CouponResponse>), (StatusCode, Json<AuthErrorResponse>)> {
    let tenant_id = auth_user.tenant_id.0;
    let code = payload.code.trim().to_uppercase();

    if code.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: "Coupon code cannot be empty".to_string(),
            }),
        ));
    }

    let discount_type = payload.discount_type.to_uppercase();
    if discount_type != "PERCENTAGE" && discount_type != "FIXED_AMOUNT" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: "Discount type must be 'PERCENTAGE' or 'FIXED_AMOUNT'".to_string(),
            }),
        ));
    }

    if payload.discount_value <= Decimal::ZERO {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse {
                error: "Discount value must be greater than zero".to_string(),
            }),
        ));
    }

    let now = Utc::now();
    let coupon_id = Uuid::new_v4();

    let created = coupon::ActiveModel {
        id: Set(coupon_id),
        tenant_id: Set(tenant_id),
        code: Set(code),
        discount_type: Set(discount_type),
        discount_value: Set(payload.discount_value),
        min_order_amount: Set(payload.min_order_amount),
        max_discount_amount: Set(payload.max_discount_amount),
        usage_limit: Set(payload.usage_limit),
        used_count: Set(0),
        valid_from: Set(payload.valid_from),
        valid_until: Set(payload.valid_until),
        is_active: Set(payload.is_active.unwrap_or(true)),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok((StatusCode::CREATED, Json(CouponResponse::from(created))))
}

#[utoipa::path(
    get,
    path = "/api/v1/promotions/coupons",
    tag = "Promotions",
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
    params(ListCouponsQuery),
    responses(
        (status = 200, description = "List of promotions coupons", body = Vec<CouponResponse>)
    )
)]
pub async fn list_coupons(
    State(state): State<SharedState>,
    auth_user: AuthUser,
    Query(params): Query<ListCouponsQuery>,
) -> Result<Json<Vec<CouponResponse>>, (StatusCode, Json<AuthErrorResponse>)> {
    let tenant_id = auth_user.tenant_id.0;
    let mut query = coupon::Entity::find()
        .filter(coupon::Column::TenantId.eq(tenant_id))
        .order_by_desc(coupon::Column::CreatedAt);

    if let Some(true) = params.active_only {
        query = query.filter(coupon::Column::IsActive.eq(true));
    }

    let coupons = query.all(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(
        coupons.into_iter().map(CouponResponse::from).collect(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/promotions/coupons/validate",
    tag = "Promotions",
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
    request_body = ValidateCouponRequest,
    responses(
        (status = 200, description = "Coupon validation outcome", body = ValidateCouponResponse)
    )
)]
pub async fn validate_coupon(
    State(state): State<SharedState>,
    auth_user: AuthUser,
    Json(payload): Json<ValidateCouponRequest>,
) -> Result<Json<ValidateCouponResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let tenant_id = auth_user.tenant_id.0;
    let code = payload.code.trim().to_uppercase();
    let now = Utc::now();

    let coupon_opt = coupon::Entity::find()
        .filter(coupon::Column::TenantId.eq(tenant_id))
        .filter(coupon::Column::Code.eq(&code))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    match coupon_opt {
        Some(m) => {
            let (is_valid, discount_amount, message) =
                calculate_discount(&m, payload.subtotal, now);
            let final_subtotal = (payload.subtotal - discount_amount).max(Decimal::ZERO);

            Ok(Json(ValidateCouponResponse {
                is_valid,
                code: m.code,
                discount_type: Some(m.discount_type),
                discount_value: Some(m.discount_value),
                discount_amount,
                final_subtotal,
                message,
            }))
        }
        None => Ok(Json(ValidateCouponResponse {
            is_valid: false,
            code,
            discount_type: None,
            discount_value: None,
            discount_amount: Decimal::ZERO,
            final_subtotal: payload.subtotal,
            message: "Coupon code does not exist".to_string(),
        })),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/promotions/coupons/{code}/redeem",
    tag = "Promotions",
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
    params(
        ("code" = String, Path, description = "Coupon code")
    ),
    request_body = RedeemCouponRequest,
    responses(
        (status = 200, description = "Coupon successfully redeemed", body = RedeemCouponResponse),
        (status = 400, description = "Invalid or expired coupon", body = AuthErrorResponse),
        (status = 404, description = "Coupon code not found", body = AuthErrorResponse)
    )
)]
pub async fn redeem_coupon(
    State(state): State<SharedState>,
    auth_user: AuthUser,
    Path(code): Path<String>,
    Json(payload): Json<RedeemCouponRequest>,
) -> Result<Json<RedeemCouponResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let tenant_id = auth_user.tenant_id.0;
    let code_upper = code.trim().to_uppercase();
    let now = Utc::now();

    let coupon_opt = coupon::Entity::find()
        .filter(coupon::Column::TenantId.eq(tenant_id))
        .filter(coupon::Column::Code.eq(&code_upper))
        .one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    let m = match coupon_opt {
        Some(c) => c,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(AuthErrorResponse {
                    error: format!("Coupon code '{}' not found", code_upper),
                }),
            ))
        }
    };

    let (is_valid, discount_amount, message) = calculate_discount(&m, payload.subtotal, now);
    if !is_valid {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthErrorResponse { error: message }),
        ));
    }

    let updated_count = m.used_count + 1;
    let remaining = m.usage_limit.map(|limit| (limit - updated_count).max(0));

    let mut active: coupon::ActiveModel = m.into();
    active.used_count = Set(updated_count);
    active.updated_at = Set(now);
    active.update(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let final_subtotal = (payload.subtotal - discount_amount).max(Decimal::ZERO);

    Ok(Json(RedeemCouponResponse {
        is_valid: true,
        code: code_upper,
        discount_amount,
        final_subtotal,
        used_count: updated_count,
        remaining_usage: remaining,
    }))
}
