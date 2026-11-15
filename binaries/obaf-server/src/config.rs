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
    #[serde(default)]
    pub replicas: Vec<String>,
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
            .set_default("database.replicas", Vec::<String>::new())?
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

        // Allow direct DATABASE_URL / OBAF_DATABASE_URL environment variable override
        if let Ok(db_url) = env::var("DATABASE_URL").or_else(|_| env::var("OBAF_DATABASE_URL")) {
            builder = builder.set_override("database.url", db_url)?;
        }

        // Allow DATABASE_REPLICAS / OBAF_DATABASE_REPLICAS comma-separated environment variable
        if let Ok(replicas_str) =
            env::var("DATABASE_REPLICAS").or_else(|_| env::var("OBAF_DATABASE_REPLICAS"))
        {
            let list: Vec<String> = replicas_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !list.is_empty() {
                builder = builder.set_override("database.replicas", list)?;
            }
        }

        // Allow PORT / HOST environment variable overrides
        if let Ok(port_str) = env::var("PORT").or_else(|_| env::var("OBAF_SERVER_PORT")) {
            if let Ok(port) = port_str.parse::<u16>() {
                builder = builder.set_override("server.port", port)?;
            }
        }
        if let Ok(host) = env::var("HOST").or_else(|_| env::var("OBAF_SERVER_HOST")) {
            builder = builder.set_override("server.host", host)?;
        }

        // Allow JWT secret overrides
        if let Ok(secret) = env::var("JWT_SECRET").or_else(|_| env::var("OBAF_SECURITY_JWT_SECRET"))
        {
            builder = builder.set_override("security.jwt_secret", secret)?;
        }
        if let Ok(exp_str) = env::var("OBAF_SECURITY_TOKEN_EXPIRY_HOURS") {
            if let Ok(exp) = exp_str.parse::<u64>() {
                builder = builder.set_override("security.token_expiry_hours", exp)?;
            }
        }

        // Allow OBAF_ prefix environment overrides (e.g. OBAF_SERVER__PORT)
        builder = builder.add_source(
            config::Environment::with_prefix("OBAF")
                .separator("__")
                .ignore_empty(true),
        );

        let config = builder.build()?.try_deserialize::<AppConfig>()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_loading() {
        let cfg = AppConfig::load().expect("Default config should load successfully");
        assert!(!cfg.server.host.is_empty());
        assert!(cfg.server.port > 0);
        assert!(!cfg.database.url.is_empty());
    }

    #[test]
    fn test_replicas_env_parsing() {
        env::set_var("DATABASE_REPLICAS", "sqlite://rep1.db, sqlite://rep2.db");
        let cfg = AppConfig::load().expect("Config should load with replicas");
        assert_eq!(cfg.database.replicas.len(), 2);
        assert_eq!(cfg.database.replicas[0], "sqlite://rep1.db");
        assert_eq!(cfg.database.replicas[1], "sqlite://rep2.db");
        env::remove_var("DATABASE_REPLICAS");
    }
}
