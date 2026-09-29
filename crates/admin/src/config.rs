use anyhow::Context;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub database: DatabaseConfig,
    #[serde(default)]
    pub bootstrap: BootstrapConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub bind_addr: String,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    #[serde(default = "default_access_ttl_secs")]
    pub access_token_ttl_secs: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BootstrapConfig {
    #[serde(default = "default_admin_username")]
    pub admin_username: String,
    #[serde(default = "default_admin_password")]
    pub admin_password: String,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            admin_username: default_admin_username(),
            admin_password: default_admin_password(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

fn default_data_dir() -> String {
    "./data".to_string()
}
fn default_access_ttl_secs() -> i64 {
    3600
}
fn default_admin_username() -> String {
    "admin".to_string()
}
fn default_admin_password() -> String {
    "changeme".to_string()
}
fn default_log_level() -> String {
    "info".to_string()
}

impl AppConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("read config {}", path.display()))?;
        let cfg: AppConfig = toml::from_str(&raw)
            .with_context(|| format!("parse config {}", path.display()))?;
        if cfg.auth.jwt_secret.len() < 32 {
            anyhow::bail!("auth.jwt_secret must be at least 32 characters");
        }
        if cfg.auth.jwt_secret.starts_with("REPLACE") {
            anyhow::bail!(
                "auth.jwt_secret is still the placeholder value; replace it in {}",
                path.display()
            );
        }
        Ok(cfg)
    }
}