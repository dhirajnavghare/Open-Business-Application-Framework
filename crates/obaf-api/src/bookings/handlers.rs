use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use obaf_database::entities::booking;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::extractor::{AuthErrorResponse, AuthUser};
use crate::state::SharedState;

#[derive(Deserialize, ToSchema)]
pub struct CreateBookingRequest {
    pub customer_id: Option<Uuid>,
    pub resource_name: String, // "Table #4", "Room 101", "Dr. Slot", "Banquet Hall"
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub notes: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateBookingStatusRequest {
    pub status: String,
}

#[derive(Serialize, ToSchema)]
pub struct BookingResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub customer_id: Option<Uuid>,
    pub resource_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<booking::Model> for BookingResponse {
    fn from(m: booking::Model) -> Self {
        Self {
            id: m.id,
            tenant_id: m.tenant_id,
            customer_id: m.customer_id,
            resource_name: m.resource_name,
            start_time: m.start_time,
            end_time: m.end_time,
            status: m.status,
            notes: m.notes,
            created_at: m.created_at,
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/bookings",
    tag = "Bookings",
    security(("bearer_auth" = [])),
    request_body = CreateBookingRequest,
    responses(
        (status = 201, description = "Booking created successfully", body = BookingResponse),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn create_booking(
    State(state): State<SharedState>,
    auth: AuthUser,
    Json(payload): Json<CreateBookingRequest>,
) -> Result<(StatusCode, Json<BookingResponse>), (StatusCode, Json<AuthErrorResponse>)> {
    let booking_id = Uuid::new_v4();
    let now = Utc::now();

    let new_booking = booking::ActiveModel {
        id: Set(booking_id),
        tenant_id: Set(auth.tenant_id.as_uuid()),
        customer_id: Set(payload.customer_id),
        resource_name: Set(payload.resource_name),
        start_time: Set(payload.start_time),
        end_time: Set(payload.end_time),
        status: Set("CONFIRMED".to_string()),
        notes: Set(payload.notes),
        created_at: Set(now),
    };

    let inserted = new_booking.insert(&state.db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthErrorResponse {
                error: format!("Failed to create booking: {}", e),
            }),
        )
    })?;

    Ok((StatusCode::CREATED, Json(inserted.into())))
}

#[utoipa::path(
    get,
    path = "/api/v1/bookings",
    tag = "Bookings",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of bookings", body = Vec<BookingResponse>),
        (status = 401, description = "Unauthorized", body = AuthErrorResponse)
    )
)]
pub async fn list_bookings(
    State(state): State<SharedState>,
    auth: AuthUser,
) -> Result<Json<Vec<BookingResponse>>, (StatusCode, Json<AuthErrorResponse>)> {
    let bookings = booking::Entity::find()
        .filter(booking::Column::TenantId.eq(auth.tenant_id.as_uuid()))
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

    let resp = bookings.into_iter().map(Into::into).collect();
    Ok(Json(resp))
}

#[utoipa::path(
    post,
    path = "/api/v1/bookings/{id}/check-in",
    tag = "Bookings",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Booking ID")
    ),
    responses(
        (status = 200, description = "Booking checked in", body = BookingResponse),
        (status = 404, description = "Booking not found", body = AuthErrorResponse)
    )
)]
pub async fn check_in_booking(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<BookingResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let booking_model = booking::Entity::find()
        .filter(booking::Column::Id.eq(id))
        .filter(booking::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .one(&state.db)
        .await
        .map_err(|e| internal_err(format!("Database error: {}", e)))?
        .ok_or_else(|| not_found_err("Booking not found"))?;

    let mut active: booking::ActiveModel = booking_model.into();
    active.status = Set("CHECKED_IN".to_string());
    let updated = active
        .update(&state.db)
        .await
        .map_err(|e| internal_err(format!("Update failed: {}", e)))?;

    state.broadcast_event(
        "booking.checked_in",
        serde_json::json!({
            "booking_id": updated.id,
            "resource": updated.resource_name,
            "status": updated.status
        }),
    );

    Ok(Json(updated.into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/bookings/{id}/check-out",
    tag = "Bookings",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Booking ID")
    ),
    responses(
        (status = 200, description = "Booking checked out / completed", body = BookingResponse),
        (status = 404, description = "Booking not found", body = AuthErrorResponse)
    )
)]
pub async fn check_out_booking(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<BookingResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let booking_model = booking::Entity::find()
        .filter(booking::Column::Id.eq(id))
        .filter(booking::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .one(&state.db)
        .await
        .map_err(|e| internal_err(format!("Database error: {}", e)))?
        .ok_or_else(|| not_found_err("Booking not found"))?;

    let mut active: booking::ActiveModel = booking_model.into();
    active.status = Set("COMPLETED".to_string());
    let updated = active
        .update(&state.db)
        .await
        .map_err(|e| internal_err(format!("Update failed: {}", e)))?;

    state.broadcast_event(
        "booking.checked_out",
        serde_json::json!({
            "booking_id": updated.id,
            "resource": updated.resource_name,
            "status": updated.status
        }),
    );

    Ok(Json(updated.into()))
}

#[utoipa::path(
    get,
    path = "/api/v1/bookings/{id}",
    tag = "Bookings",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Booking ID")
    ),
    responses(
        (status = 200, description = "Booking details", body = BookingResponse),
        (status = 404, description = "Booking not found", body = AuthErrorResponse)
    )
)]
pub async fn get_booking(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<BookingResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let booking_model = booking::Entity::find()
        .filter(booking::Column::Id.eq(id))
        .filter(booking::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .one(&state.db)
        .await
        .map_err(|e| internal_err(format!("Database error: {}", e)))?
        .ok_or_else(|| not_found_err("Booking not found"))?;

    Ok(Json(booking_model.into()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/bookings/{id}/status",
    tag = "Bookings",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Booking ID")
    ),
    request_body = UpdateBookingStatusRequest,
    responses(
        (status = 200, description = "Booking status updated", body = BookingResponse),
        (status = 404, description = "Booking not found", body = AuthErrorResponse)
    )
)]
pub async fn update_booking_status(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateBookingStatusRequest>,
) -> Result<Json<BookingResponse>, (StatusCode, Json<AuthErrorResponse>)> {
    let booking_model = booking::Entity::find()
        .filter(booking::Column::Id.eq(id))
        .filter(booking::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .one(&state.db)
        .await
        .map_err(|e| internal_err(format!("Database error: {}", e)))?
        .ok_or_else(|| not_found_err("Booking not found"))?;

    let mut active: booking::ActiveModel = booking_model.into();
    active.status = Set(payload.status);
    let updated = active
        .update(&state.db)
        .await
        .map_err(|e| internal_err(format!("Update failed: {}", e)))?;

    state.broadcast_event(
        "booking.status_updated",
        serde_json::json!({
            "booking_id": updated.id,
            "resource": updated.resource_name,
            "status": updated.status
        }),
    );

    Ok(Json(updated.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/bookings/{id}",
    tag = "Bookings",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Booking ID")
    ),
    responses(
        (status = 200, description = "Booking cancelled / deleted"),
        (status = 404, description = "Booking not found", body = AuthErrorResponse)
    )
)]
pub async fn delete_booking(
    State(state): State<SharedState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<AuthErrorResponse>)> {
    let booking_model = booking::Entity::find()
        .filter(booking::Column::Id.eq(id))
        .filter(booking::Column::TenantId.eq(auth.tenant_id.as_uuid()))
        .one(&state.db)
        .await
        .map_err(|e| internal_err(format!("Database error: {}", e)))?
        .ok_or_else(|| not_found_err("Booking not found"))?;

    let active: booking::ActiveModel = booking_model.into();
    active
        .delete(&state.db)
        .await
        .map_err(|e| internal_err(format!("Delete failed: {}", e)))?;

    Ok(Json(serde_json::json!({
        "status": "deleted",
        "booking_id": id
    })))
}

fn internal_err(msg: String) -> (StatusCode, Json<AuthErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(AuthErrorResponse { error: msg }),
    )
}

fn not_found_err(msg: &str) -> (StatusCode, Json<AuthErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(AuthErrorResponse {
            error: msg.to_string(),
        }),
    )
}
