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

pub mod entities;
pub mod migration;
pub mod pool;

pub use entities::*;
pub use migration::run_migrations;
pub use pool::{establish_connection, sanitize_url, ReadWriteRouter, TenantPoolRouter};
pub use sea_orm::DatabaseConnection;

#[cfg(test)]
mod compile_time_tests {
    use super::*;
    use static_assertions::assert_impl_all;

    // Compile-time static assertions ensuring database connection & routing thread-safety
    assert_impl_all!(DatabaseConnection: Send, Sync, Clone);
    assert_impl_all!(TenantPoolRouter: Send, Sync);
    assert_impl_all!(ReadWriteRouter: Send, Sync, Clone);
}
