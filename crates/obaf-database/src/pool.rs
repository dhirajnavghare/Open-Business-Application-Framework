use obaf_core::{ObafError, Result, TenantId};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::info;

pub async fn establish_connection(database_url: &str) -> Result<DatabaseConnection> {
    info!("Connecting to database: {}", sanitize_url(database_url));

    // Handle SQLite file directory creation if needed
    if database_url.starts_with("sqlite://") {
        if let Some(path_str) = database_url
            .strip_prefix("sqlite://")
            .and_then(|s| s.split('?').next())
        {
            if path_str != ":memory:" {
                let path = std::path::Path::new(path_str);
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() && !parent.exists() {
                        std::fs::create_dir_all(parent).map_err(|e| ObafError::Database {
                            message: format!(
                                "Failed to create SQLite directory {}: {}",
                                parent.display(),
                                e
                            ),
                        })?;
                    }
                }
            }
        }
    }

    let mut opt = ConnectOptions::new(database_url.to_owned());
    opt.max_connections(50)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .sqlx_logging(false);

    let db = Database::connect(opt)
        .await
        .map_err(|e| ObafError::Database {
            message: format!("Database connection error: {}", e),
        })?;

    info!("Successfully connected to database");
    Ok(db)
}

fn sanitize_url(url: &str) -> String {
    // Redact password if present: postgres://user:password@host/db -> postgres://user:***@host/db
    if let Some(at_idx) = url.find('@') {
        if let Some(colon_idx) = url[..at_idx].rfind(':') {
            if let Some(slash_idx) = url.find("://") {
                if colon_idx > slash_idx + 2 {
                    return format!("{}:***{}", &url[..colon_idx], &url[at_idx..]);
                }
            }
        }
    }
    url.to_string()
}

/// Thread-safe multi-tenant database router supporting dedicated per-tenant databases
/// alongside the shared cluster database pool.
#[derive(Clone)]
pub struct TenantPoolRouter {
    default_pool: DatabaseConnection,
    dedicated_pools: Arc<RwLock<HashMap<TenantId, DatabaseConnection>>>,
}

impl TenantPoolRouter {
    pub fn new(default_pool: DatabaseConnection) -> Self {
        Self {
            default_pool,
            dedicated_pools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an isolated, dedicated database connection pool for an enterprise tenant.
    pub fn register_tenant_pool(
        &self,
        tenant_id: TenantId,
        pool: DatabaseConnection,
    ) -> Result<()> {
        let mut lock = self
            .dedicated_pools
            .write()
            .map_err(|_| ObafError::Internal {
                message: "TenantPoolRouter lock poisoned".into(),
            })?;
        lock.insert(tenant_id, pool);
        Ok(())
    }

    /// Retrieve the appropriate connection for a tenant: returns dedicated pool if registered,
    /// otherwise falls back to the primary shared database pool.
    pub fn get_connection(&self, tenant_id: TenantId) -> DatabaseConnection {
        let lock = self
            .dedicated_pools
            .read()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(dedicated) = lock.get(&tenant_id) {
            dedicated.clone()
        } else {
            self.default_pool.clone()
        }
    }

    /// Get reference to the default shared cluster database pool.
    pub fn default_connection(&self) -> &DatabaseConnection {
        &self.default_pool
    }
}
