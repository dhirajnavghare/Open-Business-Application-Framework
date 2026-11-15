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

            // Semantic validation: preset_id must match filename stem
            let expected_id = path.file_stem().unwrap().to_str().unwrap();
            let preset_id = json["preset_id"].as_str().unwrap_or("");
            if preset_id != expected_id {
                panic!(
                    "Build-time test failure: Preset file {} has preset_id '{}' which does not match filename '{}'",
                    path.display(),
                    preset_id,
                    expected_id
                );
            }

            // Semantic validation: active_modules must be a non-empty list of strings
            let modules = json["active_modules"].as_array().unwrap_or_else(|| {
                panic!(
                    "Build-time test failure: Preset file {} active_modules must be an array",
                    path.display()
                )
            });
            if modules.is_empty() {
                panic!(
                    "Build-time test failure: Preset file {} active_modules cannot be empty",
                    path.display()
                );
            }

            // Semantic validation: workflow_states must contain at least 2 states
            let states = json["workflow_states"].as_array().unwrap_or_else(|| {
                panic!(
                    "Build-time test failure: Preset file {} workflow_states must be an array",
                    path.display()
                )
            });
            if states.len() < 2 {
                panic!(
                    "Build-time test failure: Preset file {} workflow_states must contain at least 2 states",
                    path.display()
                );
            }

            // Semantic validation: tax_rates (if present) must have valid name and 0.0 <= rate <= 1.0
            if let Some(tax_rates) = json.get("tax_rates").and_then(|t| t.as_array()) {
                for tax in tax_rates {
                    let name = tax.get("name").and_then(|n| n.as_str());
                    let rate = tax.get("rate").and_then(|r| r.as_f64());
                    if name.is_none() || name.unwrap().trim().is_empty() {
                        panic!(
                            "Build-time test failure: Preset file {} has tax_rate with invalid name",
                            path.display()
                        );
                    }
                    if rate.is_none() || rate.unwrap() < 0.0 || rate.unwrap() > 1.0 {
                        panic!(
                            "Build-time test failure: Preset file {} has tax_rate with invalid rate (must be 0.0 to 1.0)",
                            path.display()
                        );
                    }
                }
            }

            preset_count += 1;
        }
    }

    if preset_count < 22 {
        panic!(
            "Build-time test failure: Expected at least 22 preset templates in config/presets, found {}",
            preset_count
        );
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
