use clap::Parser;
use obaf_database::{entities::organization, establish_connection, run_migrations};
use sea_orm::EntityTrait;
use std::time::Duration;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "obaf-worker")]
#[command(about = "Dedicated background worker and scheduler daemon for OBAF")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct WorkerArgs {
    #[arg(
        short,
        long,
        default_value = "sqlite://data/obaf.db?mode=rwc",
        env = "DATABASE_URL"
    )]
    database_url: String,

    #[arg(short, long, default_value_t = 30, env = "OBAF_WORKER_INTERVAL_SECS")]
    interval_secs: u64,

    /// Execute a single maintenance and reconciliation sweep and exit (for CronJobs)
    #[arg(long, default_value_t = false)]
    once: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "obaf_worker=info,obaf_database=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = WorkerArgs::parse();

    info!(
        "Starting OBAF Background Worker v{}...",
        env!("CARGO_PKG_VERSION")
    );
    info!("Database target: {}", args.database_url);

    let db = establish_connection(&args.database_url)
        .await
        .map_err(|e| {
            error!("Database connection failed: {}", e);
            anyhow::anyhow!(e)
        })?;

    run_migrations(&db).await.map_err(|e| {
        error!("Database migration failure: {}", e);
        anyhow::anyhow!(e)
    })?;

    if args.once {
        info!("Running single background maintenance sweep...");
        run_maintenance_sweep(&db).await?;
        info!("Single sweep finished successfully.");
        return Ok(());
    }

    info!(
        "Worker daemon active. Sweep interval: {} seconds. Listening for shutdown signals...",
        args.interval_secs
    );

    let mut interval = tokio::time::interval(Duration::from_secs(args.interval_secs));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Err(e) = run_maintenance_sweep(&db).await {
                    error!("Maintenance sweep failed: {}", e);
                }
            }
            _ = shutdown_signal() => {
                info!("Shutdown signal received. Completing current worker cycle and exiting gracefully...");
                break;
            }
        }
    }

    info!("OBAF Background Worker terminated cleanly.");
    Ok(())
}

async fn run_maintenance_sweep(db: &obaf_database::DatabaseConnection) -> anyhow::Result<()> {
    // 1. Verify database responsiveness
    db.ping()
        .await
        .map_err(|e| anyhow::anyhow!("Database ping failed during worker maintenance: {}", e))?;

    // 2. Count active tenants
    let orgs = organization::Entity::find().all(db).await?;
    info!(
        "Worker maintenance sweep: {} registered tenants active. System healthy.",
        orgs.len()
    );

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
            info!("Received SIGINT (Ctrl+C). Initiating worker shutdown...");
        },
        _ = terminate => {
            info!("Received SIGTERM. Initiating worker shutdown...");
        },
    }
}
