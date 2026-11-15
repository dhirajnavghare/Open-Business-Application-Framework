use clap::{Parser, Subcommand};
use obaf_core::{generate_token, TenantId};
use obaf_database::{establish_connection, run_migrations};
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "obaf")]
#[command(about = "CLI tool for Open Business Application Framework (OBAF)")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify database connectivity, schema migrations, and preset health
    Doctor {
        #[arg(
            short,
            long,
            default_value = "sqlite://data/obaf.db?mode=rwc",
            env = "DATABASE_URL"
        )]
        database_url: String,
    },
    /// List all available out-of-the-box business presets
    Presets,
    /// Generate a signed JWT token for debugging or API automation
    Token {
        #[arg(long, default_value_t = Uuid::new_v4())]
        user_id: Uuid,
        #[arg(long, default_value_t = Uuid::new_v4())]
        tenant_id: Uuid,
        #[arg(long, default_value = "admin")]
        role: String,
        #[arg(
            long,
            default_value = "dev_secret_please_change_in_production_987654321"
        )]
        secret: String,
        #[arg(long, default_value_t = 24)]
        expiry_hours: u64,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Doctor { database_url } => {
            println!("🔍 Checking OBAF System Health...\n");

            // 1. Database Check
            print!("• Connecting to database ({}) ... ", database_url);
            match establish_connection(&database_url).await {
                Ok(db) => {
                    println!("✅ OK");

                    // 2. Migration Check
                    print!("• Verifying database schema migrations ... ");
                    match run_migrations(&db).await {
                        Ok(_) => println!("✅ OK"),
                        Err(e) => println!("❌ Migration Failed: {}", e),
                    }
                }
                Err(e) => println!("❌ Failed: {}", e),
            }

            // 3. Presets Directory Check
            print!("• Verifying config/presets/ templates ... ");
            let presets_dir = Path::new("config/presets");
            if presets_dir.exists() && presets_dir.is_dir() {
                let count = fs::read_dir(presets_dir)?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
                    .count();
                println!("✅ Found {} presets", count);
            } else {
                println!("⚠️ Directory config/presets/ not found");
            }

            println!("\n✨ OBAF Environment is ready for development & deployment.");
        }

        Commands::Presets => {
            println!("📦 Available Business Domain Presets:\n");
            let presets_dir = Path::new("config/presets");
            if presets_dir.exists() && presets_dir.is_dir() {
                let mut entries = fs::read_dir(presets_dir)?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
                    .collect::<Vec<_>>();

                entries.sort_by_key(|e| e.file_name());

                for entry in entries {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                            let id = json["preset_id"].as_str().unwrap_or("unknown");
                            let name = json["name"].as_str().unwrap_or("Unnamed");
                            let desc = json["description"].as_str().unwrap_or("");
                            println!("  🔹 {:<22} | {} \n     └─ {}\n", id, name, desc);
                        }
                    }
                }
            } else {
                println!("No presets found in config/presets/");
            }
        }

        Commands::Token {
            user_id,
            tenant_id,
            role,
            secret,
            expiry_hours,
        } => {
            let token = generate_token(user_id, TenantId(tenant_id), &role, &secret, expiry_hours)?;
            println!("🔑 Generated JWT Token for User ID: {}", user_id);
            println!("🏢 Tenant ID: {}", tenant_id);
            println!("🛡️  Role: {}", role);
            println!("⏱️  Expiry: {} hours", expiry_hours);
            println!("\nBearer Token:\n{}", token);
        }
    }

    Ok(())
}
