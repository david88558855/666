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
    "admin".to_string()
}
fn default_log_level() -> String {
    "info".to_string()
}

fn random_hex(n_bytes: usize) -> String {
    use rand::RngCore;
    use std::fmt::Write;
    let mut bytes = vec![0u8; n_bytes];
    rand::thread_rng().fill_bytes(&mut bytes);
    let mut s = String::with_capacity(n_bytes * 2);
    for b in bytes {
        let _ = write!(&mut s, "{b:02x}");
    }
    s
}

const DEFAULT_CONFIG_TEMPLATE: &str = r#"# gostc-rs-admin configuration
# Auto-generated on first start. Edit and restart to apply.

[server]
bind_addr = "0.0.0.0:8080"
data_dir = "./data"

[auth]
# At least 32 characters.
jwt_secret = "{JWT_SECRET}"
access_token_ttl_secs = 3600

[database]
url = "sqlite://./data/gostc-rs.db?mode=rwc"

[bootstrap]
# Applied only when the users table is empty (first run).
admin_username = "admin"
admin_password = "admin"

[logging]
level = "info"
"#;

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

    /// Load the config from `path`. If the file does not exist, a default
    /// config with a random `jwt_secret` is generated and written to `path`,
    /// so the binary works out of the box (default login: admin / admin).
    /// Returns the config plus a flag telling whether it was just created.
    pub fn load_or_create(path: &Path) -> anyhow::Result<(Self, bool)> {
        if path.exists() {
            return Ok((Self::load(path)?, false));
        }
        let raw = DEFAULT_CONFIG_TEMPLATE.replace("{JWT_SECRET}", &random_hex(32));
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("create dir {}", parent.display()))?;
            }
        }
        std::fs::write(path, &raw)
            .with_context(|| format!("write generated config {}", path.display()))?;
        let cfg: AppConfig = toml::from_str(&raw)
            .with_context(|| format!("parse generated config {}", path.display()))?;
        Ok((cfg, true))
    }
}