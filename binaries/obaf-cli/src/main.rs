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
    /// Scaffold a new business domain preset template in config/presets/
    NewPreset {
        #[arg(help = "Unique identifier of the preset (e.g. salon_spa, dental_clinic)")]
        name: String,
        #[arg(long, help = "Human-readable display name")]
        title: Option<String>,
    },
    /// Create a timestamped backup of the SQLite database
    Backup {
        #[arg(short, long, default_value = "data/obaf.db")]
        source: String,
        #[arg(short, long)]
        destination: Option<String>,
    },
    /// Safely restore the database from a backup file
    Restore {
        #[arg(short, long, help = "Path to the backup file to restore from")]
        source: String,
        #[arg(short, long, default_value = "data/obaf.db")]
        destination: String,
    },
    /// Run database schema migrations explicitly against target database
    Migrate {
        #[arg(
            short,
            long,
            default_value = "sqlite://data/obaf.db?mode=rwc",
            env = "DATABASE_URL"
        )]
        database_url: String,
    },
    /// Seed a demo tenant with realistic catalog, customers, and admin token
    Seed {
        #[arg(help = "Preset ID to seed (e.g. restaurant_pos, pharmacy, retail_grocery)")]
        preset: String,
        #[arg(
            short,
            long,
            default_value = "sqlite://data/obaf.db?mode=rwc",
            env = "DATABASE_URL"
        )]
        database_url: String,
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

        Commands::NewPreset { name, title } => {
            let preset_id = name.to_lowercase().replace('-', "_");
            let file_path = format!("config/presets/{}.json", preset_id);
            if Path::new(&file_path).exists() {
                anyhow::bail!("Preset file {} already exists!", file_path);
            }

            let display_name = title.unwrap_or_else(|| {
                name.replace('_', " ")
                    .split_whitespace()
                    .map(|w| {
                        let mut c = w.chars();
                        match c.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            });

            let template = serde_json::json!({
                "preset_id": preset_id,
                "name": display_name,
                "version": "1.0.0",
                "description": format!("Custom domain operations and workflow preset for {}", display_name),
                "active_modules": ["customers", "catalog", "orders", "billing", "payments"],
                "default_tax_slabs": [
                    { "name": "Standard GST 18%", "rate": 0.18 }
                ],
                "workflow_states": ["pending", "confirmed", "in_progress", "completed", "cancelled"]
            });

            fs::create_dir_all("config/presets")?;
            fs::write(&file_path, serde_json::to_string_pretty(&template)?)?;
            println!("✨ Created new preset template at: {}", file_path);
        }

        Commands::Backup {
            source,
            destination,
        } => {
            let src = Path::new(&source);
            if !src.exists() {
                anyhow::bail!("Source database file {} does not exist!", source);
            }

            let dest_str = destination.unwrap_or_else(|| {
                let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                format!("data/backups/obaf_{}.db", ts)
            });

            let dest = Path::new(&dest_str);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::copy(src, dest)?;
            println!("💾 Database successfully backed up to: {}", dest_str);
        }

        Commands::Restore {
            source,
            destination,
        } => {
            let src = Path::new(&source);
            if !src.exists() {
                anyhow::bail!("Backup file {} does not exist!", source);
            }

            let dest = Path::new(&destination);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::copy(src, dest)?;
            println!(
                "♻️  Database successfully restored from {} to: {}",
                source, destination
            );
        }

        Commands::Migrate { database_url } => {
            println!("🚀 Executing OBAF database migrations on: {}", database_url);
            let db = establish_connection(&database_url).await?;
            run_migrations(&db).await?;
            println!("✅ All migrations applied successfully!");
        }

        Commands::Seed {
            preset,
            database_url,
        } => {
            use sea_orm::ActiveModelTrait;
            let preset_id = preset.to_lowercase().replace('-', "_");
            let preset_file = format!("config/presets/{}.json", preset_id);
            if !Path::new(&preset_file).exists() {
                anyhow::bail!("Preset file {} does not exist!", preset_file);
            }

            let db = establish_connection(&database_url).await?;
            run_migrations(&db).await?;

            let ts = chrono::Utc::now().timestamp();
            let org_id = Uuid::new_v4();
            let slug = format!("seed-{}-{}", preset_id, &org_id.simple().to_string()[..6]);
            let org_name = format!("Demo {}", preset_id.replace('_', " "));
            let now = chrono::Utc::now();

            let new_org = obaf_database::entities::organization::ActiveModel {
                id: sea_orm::Set(org_id),
                slug: sea_orm::Set(slug.clone()),
                name: sea_orm::Set(org_name.clone()),
                preset: sea_orm::Set(preset_id.clone()),
                active_modules: sea_orm::Set(
                    "[\"catalog\",\"inventory\",\"orders\",\"billing\",\"customers\"]".into(),
                ),
                settings: sea_orm::Set(r#"{"seeded": true}"#.into()),
                created_at: sea_orm::Set(now),
                updated_at: sea_orm::Set(now),
            };
            new_org.insert(&db).await?;

            let user_id = Uuid::new_v4();
            let admin_email = format!("admin@demo-{}.test", preset_id);
            let password_hash = obaf_core::hash_password("DemoPassword123!")?;

            let new_user = obaf_database::entities::user::ActiveModel {
                id: sea_orm::Set(user_id),
                organization_id: sea_orm::Set(org_id),
                email: sea_orm::Set(admin_email.clone()),
                password_hash: sea_orm::Set(password_hash),
                full_name: sea_orm::Set("Demo Administrator".into()),
                role: sea_orm::Set("admin".into()),
                is_active: sea_orm::Set(true),
                created_at: sea_orm::Set(now),
                updated_at: sea_orm::Set(now),
            };
            new_user.insert(&db).await?;

            // Seed 2 sample products
            let prod1 = obaf_database::entities::product::ActiveModel {
                id: sea_orm::Set(Uuid::new_v4()),
                tenant_id: sea_orm::Set(org_id),
                category_id: sea_orm::Set(None),
                sku: sea_orm::Set(format!("DEMO-SKU-1-{}", ts)),
                name: sea_orm::Set(format!("Premium {} Sample Item", preset_id)),
                price: sea_orm::Set(rust_decimal::Decimal::from(49)),
                cost_price: sea_orm::Set(rust_decimal::Decimal::from(25)),
                tax_rate: sea_orm::Set(
                    rust_decimal::Decimal::from_str_exact("0.05").unwrap_or_default(),
                ),
                metadata: sea_orm::Set(r#"{"demo": true}"#.into()),
                is_active: sea_orm::Set(true),
                created_at: sea_orm::Set(now),
                updated_at: sea_orm::Set(now),
            };
            prod1.insert(&db).await?;

            let prod2 = obaf_database::entities::product::ActiveModel {
                id: sea_orm::Set(Uuid::new_v4()),
                tenant_id: sea_orm::Set(org_id),
                category_id: sea_orm::Set(None),
                sku: sea_orm::Set(format!("DEMO-SKU-2-{}", ts)),
                name: sea_orm::Set(format!("Standard {} Package", preset_id)),
                price: sea_orm::Set(rust_decimal::Decimal::from(99)),
                cost_price: sea_orm::Set(rust_decimal::Decimal::from(50)),
                tax_rate: sea_orm::Set(
                    rust_decimal::Decimal::from_str_exact("0.18").unwrap_or_default(),
                ),
                metadata: sea_orm::Set(r#"{"demo": true}"#.into()),
                is_active: sea_orm::Set(true),
                created_at: sea_orm::Set(now),
                updated_at: sea_orm::Set(now),
            };
            prod2.insert(&db).await?;

            // Seed a customer
            let cust = obaf_database::entities::customer::ActiveModel {
                id: sea_orm::Set(Uuid::new_v4()),
                tenant_id: sea_orm::Set(org_id),
                name: sea_orm::Set("Jane Doe (VIP Client)".into()),
                phone: sea_orm::Set(Some("+1-555-0100".into())),
                email: sea_orm::Set(Some("jane.doe@client.test".into())),
                tax_id: sea_orm::Set(None),
                address: sea_orm::Set(Some("100 Innovation Way".into())),
                metadata: sea_orm::Set(r#"{"tier": "gold"}"#.into()),
                created_at: sea_orm::Set(now),
                updated_at: sea_orm::Set(now),
            };
            cust.insert(&db).await?;

            let token = generate_token(
                user_id,
                TenantId(org_id),
                "admin",
                "dev_secret_please_change_in_production_987654321",
                24,
            )?;

            println!(
                "🌱 Successfully seeded demo tenant with preset: {}",
                preset_id
            );
            println!("🏢 Organization:    {} ({})", org_name, slug);
            println!("👤 Admin Email:    {}", admin_email);
            println!("🔑 Admin Password: DemoPassword123!");
            println!("\nBearer Token (24h valid):\n{}", token);
        }
    }

    Ok(())
}
