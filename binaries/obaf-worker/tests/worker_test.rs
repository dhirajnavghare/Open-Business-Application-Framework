use std::fs;
use std::path::Path;
use std::process::Command;

fn get_workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

#[test]
fn test_worker_help() {
    let bin = env!("CARGO_BIN_EXE_obaf-worker");
    let output = Command::new(bin)
        .current_dir(get_workspace_root())
        .arg("--help")
        .output()
        .expect("Failed to execute obaf-worker --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dedicated background worker"));
    assert!(stdout.contains("--once"));
    assert!(stdout.contains("--interval-secs"));
}

#[test]
fn test_worker_single_sweep() {
    let db_path = "/tmp/test_worker_suite.db";
    let _ = fs::remove_file(db_path);

    let bin = env!("CARGO_BIN_EXE_obaf-worker");
    let db_url = format!("sqlite://{}?mode=rwc", db_path);

    let output = Command::new(bin)
        .current_dir(get_workspace_root())
        .args(["--database-url", &db_url, "--once"])
        .output()
        .expect("Failed to execute obaf-worker --once");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Single sweep finished successfully"));

    let _ = fs::remove_file(db_path);
}

#[tokio::test]
async fn test_worker_sweep_with_active_tenants() {
    use obaf_database::entities::organization;
    use sea_orm::{ActiveModelTrait, Set};
    use uuid::Uuid;

    let db_path = "/tmp/test_worker_active_tenants.db";
    let _ = fs::remove_file(db_path);

    let db_url = format!("sqlite://{}?mode=rwc", db_path);

    // 1. Setup DB and seed active tenant
    let db = obaf_database::establish_connection(&db_url).await.unwrap();
    obaf_database::run_migrations(&db).await.unwrap();

    let org = organization::ActiveModel {
        id: Set(Uuid::new_v4()),
        name: Set("Worker Test Pharmacy".to_string()),
        slug: Set("worker-test-pharmacy".to_string()),
        preset: Set("pharmacy".to_string()),
        active_modules: Set("[\"catalog\"]".to_string()),
        settings: Set("{}".to_string()),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
    };
    org.insert(&db).await.unwrap();

    // 2. Execute worker single sweep
    let bin = env!("CARGO_BIN_EXE_obaf-worker");
    let output = Command::new(bin)
        .current_dir(get_workspace_root())
        .args(["--database-url", &db_url, "--once"])
        .output()
        .expect("Failed to execute obaf-worker --once");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Worker maintenance sweep: 1 registered tenants active"));
    assert!(stdout.contains("Single sweep finished successfully"));

    let _ = fs::remove_file(db_path);
}
