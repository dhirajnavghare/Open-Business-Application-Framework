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
    let state = Arc::new(AppState::with_router(db, router, cfg.security.jwt_secret));
    let app = create_router(state);

    // 5. Print startup banner with direct links
    print_banner(&bind_addr);

    // 6. Bind listener and serve with graceful shutdown (System Design Primer)
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
 📖 Swagger UI Docs:   http://{}/swagger-ui
 ⚡ OpenAPI JSON Spec: http://{}/api/v1/openapi.json
 🩺 Health & Probes:   http://{}/health (/live, /ready)
 ────────────────────────────────────────────────────────────
"#,
        env!("CARGO_PKG_VERSION"),
        bind_addr,
        bind_addr,
        bind_addr,
        bind_addr
    );
}
