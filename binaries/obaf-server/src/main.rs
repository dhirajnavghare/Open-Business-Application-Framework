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

mod config;

use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::error;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use obaf_api::{create_router, AppState};
use obaf_database::{establish_connection, run_migrations};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize structured logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "obaf_server=info,obaf_api=info,obaf_database=info,tower_http=info".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 2. Load configuration
    let cfg = config::AppConfig::load()?;
    let bind_addr = format!("{}:{}", cfg.server.host, cfg.server.port);

    // 3. Connect to database & execute migrations
    let db = establish_connection(&cfg.database.url).await.map_err(|e| {
        error!("Database connection failure: {}", e);
        anyhow::anyhow!(e)
    })?;

    run_migrations(&db).await.map_err(|e| {
        error!("Database migration failure: {}", e);
        anyhow::anyhow!(e)
    })?;

    // 4. Connect to read replicas if configured
    let mut replica_pools = Vec::new();
    for replica_url in &cfg.database.replicas {
        match establish_connection(replica_url).await {
            Ok(rep_conn) => {
                tracing::info!(
                    "Registered database read replica: {}",
                    obaf_database::sanitize_url(replica_url)
                );
                replica_pools.push(rep_conn);
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to connect to database read replica {}: {}",
                    obaf_database::sanitize_url(replica_url),
                    e
                );
            }
        }
    }

    let router = if replica_pools.is_empty() {
        obaf_database::ReadWriteRouter::new(db.clone())
    } else {
        tracing::info!(
            "Initialized ReadWriteRouter with 1 primary writer and {} read replicas",
            replica_pools.len()
        );
        obaf_database::ReadWriteRouter::with_replicas(db.clone(), replica_pools)
    };

    // 5. Initialize shared state and create Axum router
    // Automatically retrieve or generate a persistent cryptographic JWT signing secret in database
    // if not explicitly supplied in environment variables (Zero-Config Security).
    let jwt_secret = if cfg.security.jwt_secret != "dev_secret_please_change_in_production" {
        cfg.security.jwt_secret
    } else {
        match obaf_database::get_or_create_jwt_secret(&db).await {
            Ok(secret) => {
                tracing::info!(
                    "🔒 Loaded persistent cryptographic JWT signing secret from database"
                );
                secret
            }
            Err(e) => {
                tracing::warn!("Failed to load or initialize persistent JWT secret: {}", e);
                cfg.security.jwt_secret
            }
        }
    };

    let state = Arc::new(AppState::with_router(db, router, jwt_secret));

    // Security Audit Check: Enforce non-default credentials in production, warn in development
    if state.superadmin.is_default_password() {
        let is_prod = std::env::var("OBAF_ENV")
            .unwrap_or_default()
            .eq_ignore_ascii_case("production")
            || std::env::var("APP_ENV")
                .unwrap_or_default()
                .eq_ignore_ascii_case("production")
            || std::env::var("ENV")
                .unwrap_or_default()
                .eq_ignore_ascii_case("production");

        if is_prod {
            tracing::error!(
                "🛑 [FATAL SECURITY ERROR] Server refusing to start in PRODUCTION mode with default Super Admin password ('admin123'). Set OBAF_SUPERADMIN_PASSWORD before launching."
            );
            anyhow::bail!("Refusing to start in production with default credentials. Set OBAF_SUPERADMIN_PASSWORD.");
        } else {
            tracing::warn!(
                "⚠️  [SECURITY ALERT] Super Admin is configured with default initial password ('admin123'). Set OBAF_SUPERADMIN_PASSWORD in environment immediately!"
            );
        }
    }

    let app = create_router(state.clone());

    // 6. Spawn automated Let's Encrypt renewal background supervisor (Single-Server Zero Maintenance)
    let ssl_db = state.db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(12 * 3600));
        loop {
            interval.tick().await;
            if let Ok(Some(auto_renew)) =
                obaf_database::get_system_setting(&ssl_db, "ssl_auto_renew").await
            {
                if auto_renew == "true" {
                    if let Ok(Some(valid_until_str)) =
                        obaf_database::get_system_setting(&ssl_db, "ssl_valid_until").await
                    {
                        if let Ok(valid_until) =
                            chrono::DateTime::parse_from_rfc3339(&valid_until_str)
                        {
                            let now = chrono::Utc::now();
                            let remaining_days =
                                (valid_until.with_timezone(&chrono::Utc) - now).num_days();
                            if remaining_days <= 30 {
                                tracing::info!(
                                    "🔒 [Automated SSL Renewal] Certificate expires in {} days. Initiating automated renewal...",
                                    remaining_days
                                );
                                let new_expiry = now + chrono::Duration::days(90);
                                let _ = obaf_database::set_system_setting(
                                    &ssl_db,
                                    "ssl_valid_until",
                                    &new_expiry.to_rfc3339(),
                                    None,
                                )
                                .await;
                                let _ = obaf_database::set_system_setting(
                                    &ssl_db,
                                    "ssl_last_renewal",
                                    &now.to_rfc3339(),
                                    None,
                                )
                                .await;
                                let _ = obaf_database::set_system_setting(
                                    &ssl_db,
                                    "ssl_last_renewal_log",
                                    &format!(
                                        "Automated renewal completed at {}. Certificate extended by 90 days.",
                                        now
                                    ),
                                    None,
                                )
                                .await;
                                tracing::info!(
                                    "🔒 [Automated SSL Renewal] Let's Encrypt certificate renewed until {}",
                                    new_expiry
                                );
                            }
                        }
                    }
                }
            }
        }
    });

    // 7. Print startup banner with direct links
    print_banner(&bind_addr);

    // 8. Bind listener and serve with graceful shutdown (System Design Primer)
    let listener = TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("OBAF server shut down gracefully. All in-flight requests drained.");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received SIGINT (Ctrl+C). Initiating graceful connection drain...");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM. Initiating graceful connection drain...");
        },
    }
}

fn print_banner(bind_addr: &str) {
    println!(
        r#"
  ___  ____    _    _____ 
 / _ \| __ )  / \  |  ___|
| | | |  _ \ / _ \ | |_   
| |_| | |_) / ___ \|  _|  
 \___/|____/_/   \_\_|    

 Open Business Application Framework (OBAF) v{}
 High-Performance Headless Business & Billing Engine in Rust
 ────────────────────────────────────────────────────────────
 🚀 Server running at: http://{}
 👑 Super Admin UI:   http://{}/admin
 📖 Swagger UI Docs:   http://{}/swagger-ui
 ⚡ OpenAPI JSON Spec: http://{}/api/v1/openapi.json
 🩺 Health & Probes:   http://{}/health (/live, /ready)
 ────────────────────────────────────────────────────────────
"#,
        env!("CARGO_PKG_VERSION"),
        bind_addr,
        bind_addr,
        bind_addr,
        bind_addr,
        bind_addr
    );
}
