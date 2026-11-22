pub mod handlers;

use axum::{
    routing::{get, post},
    Router,
};

use crate::state::SharedState;
use handlers::{
    create_coupon, delete_coupon, get_coupon, list_coupons, redeem_coupon, update_coupon,
    validate_coupon,
};

pub fn promotion_routes() -> Router<SharedState> {
    Router::new()
        .route("/coupons", post(create_coupon).get(list_coupons))
        .route(
            "/coupons/{code}",
            get(get_coupon).patch(update_coupon).delete(delete_coupon),
        )
        .route("/coupons/validate", post(validate_coupon))
        .route("/coupons/{code}/redeem", post(redeem_coupon))
}
