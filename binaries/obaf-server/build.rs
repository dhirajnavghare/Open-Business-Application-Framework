use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=config/presets");
    println!("cargo:rerun-if-changed=config/default.toml");

    // 1. Validate all business domain presets at compile time
    let presets_dir = if Path::new("../../config/presets").exists() {
        Path::new("../../config/presets")
    } else if Path::new("config/presets").exists() {
        Path::new("config/presets")
    } else {
        panic!("Build-time test failure: config/presets directory not found");
    };

    let entries = fs::read_dir(presets_dir).expect("Failed to read config/presets directory");
    let mut preset_count = 0;

    for entry in entries {
        let entry = entry.expect("Failed to read preset entry");
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Failed to read preset file {}: {}", path.display(), e));
            let json: serde_json::Value = serde_json::from_str(&content).unwrap_or_else(|e| {
                panic!("Preset file {} is not valid JSON: {}", path.display(), e)
            });

            // Validate mandatory preset fields
            let required_fields = [
                "preset_id",
                "name",
                "version",
                "description",
                "active_modules",
                "workflow_states",
            ];
            for field in required_fields {
                if json.get(field).is_none() {
                    panic!(
                        "Build-time test failure: Preset file {} is missing required field '{}'",
                        path.display(),
                        field
                    );
                }
            }
            preset_count += 1;
        }
    }

    if preset_count == 0 {
        panic!("Build-time test failure: No preset templates found in config/presets");
    }

    // 2. Validate baseline configuration file
    let config_file = if Path::new("../../config/default.toml").exists() {
        Path::new("../../config/default.toml")
    } else if Path::new("config/default.toml").exists() {
        Path::new("config/default.toml")
    } else {
        panic!("Build-time test failure: config/default.toml not found");
    };

    let toml_content = fs::read_to_string(config_file).expect("Failed to read config/default.toml");
    if !toml_content.contains("[server]") || !toml_content.contains("[database]") {
        panic!("Build-time test failure: config/default.toml is missing [server] or [database] sections");
    }

    println!(
        "cargo:warning=Build-time automated verification passed: {} presets verified, configuration validated.",
        preset_count
    );
}
