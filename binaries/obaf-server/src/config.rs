use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct SecurityConfig {
    pub jwt_secret: String,
    pub token_expiry_hours: u64,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub security: SecurityConfig,
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let mut builder = config::Config::builder()
            .set_default("server.host", "127.0.0.1")?
            .set_default("server.port", 8080)?
            .set_default("database.url", "sqlite://data/obaf.db?mode=rwc")?
            .set_default(
                "security.jwt_secret",
                "dev_secret_please_change_in_production",
            )?
            .set_default("security.token_expiry_hours", 24)?;

        // Try to load config/default.toml if exists
        let config_path =
            env::var("OBAF_CONFIG_PATH").unwrap_or_else(|_| "config/default.toml".to_string());
        if std::path::Path::new(&config_path).exists() {
            builder = builder.add_source(config::File::with_name(&config_path));
        }

        // Allow direct DATABASE_URL environment variable override
        if let Ok(db_url) = env::var("DATABASE_URL") {
            builder = builder.set_override("database.url", db_url)?;
        }

        // Allow PORT environment variable override
        if let Ok(port_str) = env::var("PORT") {
            if let Ok(port) = port_str.parse::<u16>() {
                builder = builder.set_override("server.port", port)?;
            }
        }

        // Allow OBAF_ prefix environment overrides
        builder = builder.add_source(
            config::Environment::with_prefix("OBAF")
                .separator("__")
                .ignore_empty(true),
        );

        let config = builder.build()?.try_deserialize::<AppConfig>()?;
        Ok(config)
    }
}
