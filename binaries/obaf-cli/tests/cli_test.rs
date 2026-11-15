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

#[test]
fn test_cli_init_project() {
    let test_dir = "/tmp/test_obaf_init_bistro";
    let _ = fs::remove_dir_all(test_dir);

    let bin = env!("CARGO_BIN_EXE_obaf-cli");
    let output = Command::new(bin)
        .current_dir(get_workspace_root())
        .args(["init", test_dir, "--preset", "restaurant_pos"])
        .output()
        .expect("Failed to execute obaf-cli init");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Successfully initialized new OBAF project"));

    assert!(Path::new(test_dir).join(".env").exists());
    assert!(Path::new(test_dir).join("docker-compose.yml").exists());
    assert!(Path::new(test_dir).join("client.ts").exists());
    assert!(Path::new(test_dir).join("client.py").exists());
    assert!(Path::new(test_dir).join("README.md").exists());

    let _ = fs::remove_dir_all(test_dir);
}

#[test]
fn test_cli_migrate() {
    let db_path = "/tmp/test_cli_migrate.db";
    let _ = fs::remove_file(db_path);

    let bin = env!("CARGO_BIN_EXE_obaf-cli");
    let db_url = format!("sqlite://{}?mode=rwc", db_path);

    let output = Command::new(bin)
        .current_dir(get_workspace_root())
        .args(["migrate", "--database-url", &db_url])
        .output()
        .expect("Failed to execute obaf-cli migrate");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Executing OBAF database migrations"));
    assert!(stdout.contains("All migrations applied successfully"));

    let _ = fs::remove_file(db_path);
}

#[test]
fn test_cli_backup_and_restore() {
    let src_path = "/tmp/test_backup_src.db";
    let dest_path = "/tmp/test_backup_dest.db";
    let restored_path = "/tmp/test_restored.db";
    let _ = fs::remove_file(src_path);
    let _ = fs::remove_file(dest_path);
    let _ = fs::remove_file(restored_path);

    fs::write(src_path, b"test-sqlite-db-content").expect("Failed to write mock db");

    let bin = env!("CARGO_BIN_EXE_obaf-cli");

    // Backup
    let backup_out = Command::new(bin)
        .current_dir(get_workspace_root())
        .args(["backup", "--source", src_path, "--destination", dest_path])
        .output()
        .expect("Failed to execute obaf-cli backup");

    assert!(backup_out.status.success());
    assert!(Path::new(dest_path).exists());

    // Restore
    let restore_out = Command::new(bin)
        .current_dir(get_workspace_root())
        .args([
            "restore",
            "--source",
            dest_path,
            "--destination",
            restored_path,
        ])
        .output()
        .expect("Failed to execute obaf-cli restore");

    assert!(restore_out.status.success());
    assert!(Path::new(restored_path).exists());
    assert_eq!(fs::read(restored_path).unwrap(), b"test-sqlite-db-content");

    let _ = fs::remove_file(src_path);
    let _ = fs::remove_file(dest_path);
    let _ = fs::remove_file(restored_path);
}

#[test]
fn test_cli_new_preset() {
    let preset_name = "test_temp_autogen_preset";
    let preset_file = get_workspace_root().join(format!("config/presets/{}.json", preset_name));
    let _ = fs::remove_file(&preset_file);

    let bin = env!("CARGO_BIN_EXE_obaf-cli");
    let output = Command::new(bin)
        .current_dir(get_workspace_root())
        .args(["new-preset", preset_name, "--title", "Temporary Auto Gen"])
        .output()
        .expect("Failed to execute obaf-cli new-preset");

    assert!(output.status.success());
    assert!(preset_file.exists());

    let content = fs::read_to_string(&preset_file).expect("Read autogen preset");
    assert!(content.contains(preset_name));
    assert!(content.contains("Temporary Auto Gen"));

    let _ = fs::remove_file(&preset_file);
}
