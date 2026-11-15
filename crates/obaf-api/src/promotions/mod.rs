pub mod handlers;

use axum::{routing::post, Router};

use crate::state::SharedState;
use handlers::{create_coupon, list_coupons, redeem_coupon, validate_coupon};

pub fn promotion_routes() -> Router<SharedState> {
    Router::new()
        .route("/coupons", post(create_coupon).get(list_coupons))
        .route("/coupons/validate", post(validate_coupon))
        .route("/coupons/:code/redeem", post(redeem_coupon))
}
