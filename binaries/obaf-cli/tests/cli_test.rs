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
fn test_cli_help() {
    let bin = env!("CARGO_BIN_EXE_obaf-cli");
    let output = Command::new(bin)
        .current_dir(get_workspace_root())
        .arg("--help")
        .output()
        .expect("Failed to execute obaf-cli --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Open Business Application Framework (OBAF)"));
    assert!(stdout.contains("doctor"));
    assert!(stdout.contains("presets"));
    assert!(stdout.contains("token"));
}

#[test]
fn test_cli_presets() {
    let bin = env!("CARGO_BIN_EXE_obaf-cli");
    let output = Command::new(bin)
        .current_dir(get_workspace_root())
        .arg("presets")
        .output()
        .expect("Failed to execute obaf-cli presets");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pharmacy"));
    assert!(stdout.contains("restaurant_pos"));
    assert!(stdout.contains("retail_grocery"));
}

#[test]
fn test_cli_token_generation() {
    let bin = env!("CARGO_BIN_EXE_obaf-cli");
    let output = Command::new(bin)
        .current_dir(get_workspace_root())
        .args(["token", "--role", "admin"])
        .output()
        .expect("Failed to execute obaf-cli token");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Bearer Token:"));
    assert!(stdout.contains("eyJ"));
}

#[test]
fn test_cli_doctor_and_migrations() {
    let db_path = "/tmp/test_cli_doctor_suite.db";
    let _ = fs::remove_file(db_path);

    let bin = env!("CARGO_BIN_EXE_obaf-cli");
    let db_url = format!("sqlite://{}?mode=rwc", db_path);

    let output = Command::new(bin)
        .current_dir(get_workspace_root())
        .args(["doctor", "--database-url", &db_url])
        .output()
        .expect("Failed to execute obaf-cli doctor");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("OK"));
    assert!(stdout.contains("Found 22 presets"));

    let _ = fs::remove_file(db_path);
}

#[test]
fn test_cli_seed_and_export() {
    let db_path = "/tmp/test_cli_seed_suite.db";
    let _ = fs::remove_file(db_path);

    let bin = env!("CARGO_BIN_EXE_obaf-cli");
    let db_url = format!("sqlite://{}?mode=rwc", db_path);

    // 1. Seed demo pharmacy tenant
    let seed_out = Command::new(bin)
        .current_dir(get_workspace_root())
        .args(["seed", "pharmacy", "--database-url", &db_url])
        .output()
        .expect("Failed to seed database");

    assert!(seed_out.status.success());
    let seed_stdout = String::from_utf8_lossy(&seed_out.stdout);
    assert!(seed_stdout.contains("Successfully seeded demo tenant with preset: pharmacy"));

    // 2. Export products
    let export_out = Command::new(bin)
        .current_dir(get_workspace_root())
        .args(["export", "products", "--database-url", &db_url])
        .output()
        .expect("Failed to export products");

    assert!(export_out.status.success());
    let export_stdout = String::from_utf8_lossy(&export_out.stdout);
    assert!(export_stdout.contains("Premium pharmacy Sample Item"));

    let _ = fs::remove_file(db_path);
}
