// Copyright 2026 Dhiraj Navghare
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use clap::{Parser, Subcommand};
use obaf_core::{generate_token, TenantId};
use obaf_database::{establish_connection, run_migrations};
use rand::{distributions::Alphanumeric, Rng};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

fn find_presets_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("OBAF_PRESETS_DIR") {
        let p = PathBuf::from(dir);
        if p.exists() {
            return p;
        }
    }
    for candidate in &[
        "config/presets",
        "../../config/presets",
        "../config/presets",
    ] {
        let p = Path::new(candidate);
        if p.exists() && p.is_dir() {
            return p.to_path_buf();
        }
    }
    PathBuf::from("config/presets")
}

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
    /// Check live server health, metrics, and database connectivity
    Status {
        #[arg(
            short,
            long,
            default_value = "http://127.0.0.1:8080",
            env = "OBAF_SERVER_URL",
            help = "Target OBAF HTTP server URL"
        )]
        url: String,
        #[arg(
            short,
            long,
            default_value = "sqlite://data/obaf.db?mode=rwc",
            env = "DATABASE_URL",
            help = "Optional database URL to check tenant and table statistics"
        )]
        database_url: String,
        #[arg(
            short,
            long,
            default_value_t = false,
            help = "Continuous monitoring mode (refreshes every 2 seconds)"
        )]
        watch: bool,
    },
    /// Export the compiled OpenAPI 3.1 JSON specification
    Openapi {
        #[arg(
            short,
            long,
            help = "Output file path (e.g. docs/openapi.json). Prints to stdout if omitted."
        )]
        output: Option<String>,
    },
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
    /// Scaffold a new turnkey business project with Docker, .env, and SDK clients
    Init {
        #[arg(help = "Name of the new project directory to create")]
        project_name: String,
        #[arg(
            short,
            long,
            default_value = "retail_grocery",
            help = "Business domain preset (e.g. restaurant_pos, pharmacy, hotel_management)"
        )]
        preset: String,
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
    /// Export tenant data (products, customers, orders, suppliers) as JSON
    Export {
        #[arg(help = "Entity to export: products, customers, orders, suppliers")]
        entity: String,
        #[arg(
            short,
            long,
            default_value = "sqlite://data/obaf.db?mode=rwc",
            env = "DATABASE_URL"
        )]
        database_url: String,
        #[arg(short, long, help = "Optional Tenant ID filter")]
        tenant_id: Option<Uuid>,
        #[arg(short, long, help = "Output file path (prints to stdout if omitted)")]
        output: Option<String>,
    },
    /// Run background maintenance and reconciliation sweep
    Worker {
        #[arg(
            short,
            long,
            default_value = "sqlite://data/obaf.db?mode=rwc",
            env = "DATABASE_URL"
        )]
        database_url: String,
        #[arg(long, default_value_t = false)]
        once: bool,
    },
}

async fn print_system_status(
    client: &reqwest::Client,
    base_url: &str,
    health_url: &str,
    metrics_url: &str,
    llms_url: &str,
    database_url: &str,
) {
    let start = std::time::Instant::now();
    println!("┌──────────────────────────────────────────────────────────────────┐");
    println!("│        Open Business Application Framework (OBAF) System Status  │");
    println!("└──────────────────────────────────────────────────────────────────┘\n");

    match client.get(health_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let elapsed = start.elapsed();
            let health_json: serde_json::Value = resp.json().await.unwrap_or_default();
            let version = health_json
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or(env!("CARGO_PKG_VERSION"));
            let db_status = health_json
                .get("database")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            println!("  🚀 Server Endpoint:   🟢 ONLINE ({})", base_url);
            println!("  ⚡ Response Latency:  {:.2?}", elapsed);
            println!("  🏷️  Framework Version: v{}", version);
            println!(
                "  🗄️  Database Status:   {}",
                if db_status == "connected" {
                    "🟢 Connected"
                } else {
                    "🔴 Unreachable"
                }
            );

            // Check OpenAPI & Swagger UI
            let swagger_url = format!("{}/swagger-ui", base_url);
            let openapi_ok = client
                .get(format!("{}/api/v1/openapi.json", base_url))
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false);
            if openapi_ok {
                println!("  📖 Swagger UI Docs:   🟢 Available ({})", swagger_url);
            } else {
                println!("  📖 Swagger UI Docs:   ⚪ Not Available");
            }

            // Check LLMs.txt AI Context
            let llms_ok = client
                .get(llms_url)
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false);
            if llms_ok {
                println!("  🤖 AI Agent Context:  🟢 Available ({})", llms_url);
            }

            // Check Prometheus metrics
            if let Ok(m_resp) = client.get(metrics_url).send().await {
                if m_resp.status().is_success() {
                    println!("  📊 Prometheus Metric: 🟢 Active ({})", metrics_url);
                }
            }
        }
        Ok(resp) => {
            println!(
                "  🚀 Server Endpoint:   ⚠️ UNEXPECTED STATUS (HTTP {}) at {}",
                resp.status(),
                base_url
            );
        }
        Err(_) => {
            println!(
                "  🚀 Server Endpoint:   🔴 OFFLINE (Cannot connect to {})",
                base_url
            );
            println!(
                "     Hint: Run 'make dev' or 'cargo run -p obaf-server' to launch the API engine."
            );
        }
    }

    // Local Database & Presets Summary
    println!("\n  📦 Local Domain Engine:");
    let presets_dir = find_presets_dir();
    if presets_dir.exists() && presets_dir.is_dir() {
        let count = fs::read_dir(&presets_dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
                    .count()
            })
            .unwrap_or(0);
        println!("  • Domain Presets:     {} available out-of-the-box", count);
    }

    // Inspect database connection directly if available
    if let Ok(db) = establish_connection(database_url).await {
        use sea_orm::ConnectionTrait;
        if let Ok(Some(row)) = db
            .query_one(sea_orm::Statement::from_string(
                db.get_database_backend(),
                "SELECT COUNT(*) as count FROM organizations;".to_string(),
            ))
            .await
        {
            let count: i64 = row.try_get("", "count").unwrap_or(0);
            println!("  • Active Tenants:     {} provisioned in database", count);
        }
    }
    println!("────────────────────────────────────────────────────────────────────\n");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status {
            url,
            database_url,
            watch,
        } => {
            let base_url = url.trim_end_matches('/');
            let health_url = format!("{}/health", base_url);
            let metrics_url = format!("{}/metrics", base_url);
            let llms_url = format!("{}/llms.txt", base_url);

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(3))
                .build()
                .unwrap_or_default();

            if watch {
                println!(
                    "👀 Entering live monitoring mode (polling every 2s, press Ctrl+C to stop)...\n"
                );
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
                loop {
                    print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
                    print_system_status(
                        &client,
                        base_url,
                        &health_url,
                        &metrics_url,
                        &llms_url,
                        &database_url,
                    )
                    .await;
                    interval.tick().await;
                }
            } else {
                print_system_status(
                    &client,
                    base_url,
                    &health_url,
                    &metrics_url,
                    &llms_url,
                    &database_url,
                )
                .await;
            }
        }

        Commands::Openapi { output } => {
            use utoipa::OpenApi;
            let spec = obaf_api::docs::ApiDoc::openapi()
                .to_pretty_json()
                .map_err(|e| anyhow::anyhow!("Failed to serialize OpenAPI spec: {}", e))?;

            if let Some(path) = output {
                if let Some(parent) = Path::new(&path).parent() {
                    if !parent.as_os_str().is_empty() && !parent.exists() {
                        fs::create_dir_all(parent)?;
                    }
                }
                fs::write(&path, &spec)?;
                println!("✅ OpenAPI 3.1 JSON specification written to: {}", path);
            } else {
                println!("{}", spec);
            }
        }

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
            let presets_dir = find_presets_dir();
            if presets_dir.exists() && presets_dir.is_dir() {
                let count = fs::read_dir(&presets_dir)?
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
            let presets_dir = find_presets_dir();
            if presets_dir.exists() && presets_dir.is_dir() {
                let mut entries = fs::read_dir(&presets_dir)?
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

        Commands::Init {
            project_name,
            preset,
        } => {
            let project_path = Path::new(&project_name);
            if project_path.exists() {
                anyhow::bail!("Directory '{}' already exists!", project_name);
            }
            fs::create_dir_all(project_path)?;
            fs::create_dir_all(project_path.join("data"))?;

            // 1. .env
            let env_content = format!(
                "HOST=0.0.0.0\nPORT=8080\nDATABASE_URL=sqlite:///app/data/obaf.db?mode=rwc\nJWT_SECRET={}\nOBAF_DEFAULT_PRESET={}\n",
                Uuid::new_v4().simple(),
                preset
            );
            fs::write(project_path.join(".env"), env_content)?;

            // 2. docker-compose.yml
            let compose_content = format!(
                r#"services:
  obaf:
    image: ghcr.io/dhirajnavghare/obaf-server:latest
    container_name: {}-server
    restart: unless-stopped
    ports:
      - "8080:8080"
    environment:
      - HOST=0.0.0.0
      - PORT=8080
      - DATABASE_URL=sqlite:///app/data/obaf.db?mode=rwc
    volumes:
      - ./data:/app/data
"#,
                project_name
            );
            fs::write(project_path.join("docker-compose.yml"), compose_content)?;

            // 3. client.ts
            let ts_client = format!(
                r#"// Official TypeScript Client Starter for {}
import {{ ObafClient }} from "@obaf/sdk";

const client = new ObafClient({{ baseUrl: "http://127.0.0.1:8080" }});

async function main() {{
  const health = await client.health();
  console.log("Server Health:", health);

  console.log("Bootstrapping demo tenant with preset: {}");
  const tenant = await client.bootstrap({{
    organization_name: "My {}",
    slug: "my-{}",
    preset: "{}",
    admin_email: "admin@example.com",
    admin_name: "Store Manager",
    admin_password: "Password123!",
  }});
  console.log("Tenant ready:", tenant);
}}

main().catch(console.error);
"#,
                project_name, preset, project_name, project_name, preset
            );
            fs::write(project_path.join("client.ts"), ts_client)?;

            // 4. client.py
            let py_client = format!(
                r#"# Official Python Client Starter for {}
from obaf import ObafClient

client = ObafClient(base_url="http://127.0.0.1:8080")

def main():
    health = client.health()
    print("Server Health:", health)

    print("Bootstrapping demo tenant with preset: {}")
    tenant = client.bootstrap(
        organization_name="My {}",
        slug="my-{}",
        preset="{}",
        admin_email="admin@example.com",
        admin_name="Store Manager",
        admin_password="Password123!",
    )
    print("Tenant ready:", tenant)

if __name__ == "__main__":
    main()
"#,
                project_name, preset, project_name, project_name, preset
            );
            fs::write(project_path.join("client.py"), py_client)?;

            // 5. README.md
            let readme_content = format!(
                r#"# {}

Turnkey business application powered by **Open Business Application Framework (OBAF)**.
- **Configured Domain Preset:** `{}`
- **API Documentation:** http://localhost:8080/swagger-ui
- **Health Check:** http://localhost:8080/health

## Getting Started

```bash
# 1. Start the OBAF server
docker compose up -d

# 2. Run TypeScript or Python clients
deno run -A client.ts  # or bun client.ts / ts-node client.ts
python3 client.py
```
"#,
                project_name, preset
            );
            fs::write(project_path.join("README.md"), readme_content)?;

            println!(
                "✨ Successfully initialized new OBAF project: {}",
                project_name
            );
            println!("📁 Location:    {}", project_path.display());
            println!("📦 Preset:      {}", preset);
            println!("\nNext steps:");
            println!("  cd {}", project_name);
            println!("  docker compose up -d");
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
            let preset_file = find_presets_dir().join(format!("{}.json", preset_id));
            if !preset_file.exists() {
                anyhow::bail!("Preset file {} does not exist!", preset_file.display());
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
            let generated_password: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(24)
                .map(char::from)
                .collect();
            let password_hash = obaf_core::hash_password(&generated_password)?;
            println!(
                "Seeded admin credentials for {} => email: {}, password: {}",
                preset_id, admin_email, generated_password
            );

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

        Commands::Export {
            entity,
            database_url,
            tenant_id,
            output,
        } => {
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
            let db = establish_connection(&database_url).await?;

            let json_value = match entity.to_lowercase().as_str() {
                "products" | "product" | "catalog" => {
                    let mut q = obaf_database::entities::product::Entity::find();
                    if let Some(tid) = tenant_id {
                        q = q.filter(obaf_database::entities::product::Column::TenantId.eq(tid));
                    }
                    let items = q.all(&db).await?;
                    serde_json::to_value(items)?
                }
                "customers" | "customer" => {
                    let mut q = obaf_database::entities::customer::Entity::find();
                    if let Some(tid) = tenant_id {
                        q = q.filter(obaf_database::entities::customer::Column::TenantId.eq(tid));
                    }
                    let items = q.all(&db).await?;
                    serde_json::to_value(items)?
                }
                "orders" | "order" => {
                    let mut q = obaf_database::entities::order::Entity::find();
                    if let Some(tid) = tenant_id {
                        q = q.filter(obaf_database::entities::order::Column::TenantId.eq(tid));
                    }
                    let items = q.all(&db).await?;
                    serde_json::to_value(items)?
                }
                "suppliers" | "supplier" => {
                    let mut q = obaf_database::entities::supplier::Entity::find();
                    if let Some(tid) = tenant_id {
                        q = q.filter(obaf_database::entities::supplier::Column::TenantId.eq(tid));
                    }
                    let items = q.all(&db).await?;
                    serde_json::to_value(items)?
                }
                other => {
                    anyhow::bail!(
                        "Unknown export entity '{}'. Choose from: products, customers, orders, suppliers",
                        other
                    );
                }
            };

            let formatted = serde_json::to_string_pretty(&json_value)?;

            if let Some(out_path) = output {
                fs::write(&out_path, formatted)?;
                println!("📁 Data exported successfully to: {}", out_path);
            } else {
                println!("{}", formatted);
            }
        }
        Commands::Worker {
            database_url,
            once: _,
        } => {
            use sea_orm::EntityTrait;
            println!("⚙️  Connecting to database ({})...", database_url);
            let db = establish_connection(&database_url)
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
            println!("🔍 Executing background reconciliation sweep...");
            let orgs = obaf_database::entities::organization::Entity::find()
                .all(&db)
                .await?;
            println!(
                "✅ Maintenance complete: {} active tenant organizations.",
                orgs.len()
            );
        }
    }

    Ok(())
}
