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

    // 4. Initialize shared state and create Axum router
    let state = Arc::new(AppState::new(db, cfg.security.jwt_secret));
    let app = create_router(state);

    // 5. Print startup banner with direct links
    print_banner(&bind_addr);

    // 6. Bind listener and serve
    let listener = TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
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
 🩺 Health Check:      http://{}/health
 ────────────────────────────────────────────────────────────
"#,
        env!("CARGO_PKG_VERSION"),
        bind_addr,
        bind_addr,
        bind_addr,
        bind_addr
    );
}
