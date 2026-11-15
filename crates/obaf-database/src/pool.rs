use obaf_core::{ObafError, Result};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
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
