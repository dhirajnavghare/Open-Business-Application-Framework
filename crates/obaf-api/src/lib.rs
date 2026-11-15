pub mod admin;
pub mod auth;
pub mod billing;
pub mod bookings;
pub mod catalog;
pub mod customers;
pub mod docs;
pub mod events;
pub mod health;
pub mod inventory;
pub mod meta;
pub mod orders;
pub mod payments;
pub mod pos;
pub mod procurement;
pub mod promotions;
pub mod reports;
pub mod router;
pub mod search;
pub mod state;
pub mod subscriptions;
pub mod suppliers;
pub mod sync;
pub mod webhooks;

pub use router::create_router;
pub use state::{AppState, SharedState};

#[cfg(test)]
mod compile_time_tests {
    use super::*;
    use static_assertions::assert_impl_all;

    // Compile-time static assertion verifying Axum AppState thread-safety invariants
    assert_impl_all!(AppState: Send, Sync, Clone);
}
