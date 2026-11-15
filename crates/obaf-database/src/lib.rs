pub mod entities;
pub mod migration;
pub mod pool;

pub use entities::*;
pub use migration::run_migrations;
pub use pool::{establish_connection, ReadWriteRouter, TenantPoolRouter};
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
