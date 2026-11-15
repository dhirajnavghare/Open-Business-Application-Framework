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
