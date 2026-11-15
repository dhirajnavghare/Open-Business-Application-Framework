// Copyright 2026 Dhiraj Navghare
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
