use crate::error::{AppError, AppResult};
use crate::models::Role;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{FromRow, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

#[derive(Clone)]
pub struct Db {
    pub pool: SqlitePool,
}

impl Db {
    pub async fn connect(url: &str) -> AppResult<Self> {
        // Ensure SQLite parent directory exists.
        if let Some(path) = url.strip_prefix("sqlite://") {
            let path = path.split('?').next().unwrap_or(path);
            if let Some(parent) = Path::new(path).parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
        }
        let pool = SqlitePoolOptions::new()
            .max_connections(16)
            .acquire_timeout(Duration::from_secs(5))
            .connect(url)
            .await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> AppResult<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    pub async fn user_count(&self) -> AppResult<i64> {
        let row: (i64,) = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    pub async fn create_user(
        &self,
        username: &str,
        password_hash: &str,
        role: Role,
        must_change_password: bool,
    ) -> AppResult<i64> {
        let role_str = role.as_str();
        let result = sqlx::query(
            "INSERT INTO users (username, password_hash, role, must_change_password)
             VALUES (?, ?, ?, ?)",
        )
        .bind(username)
        .bind(password_hash)
        .bind(role_str)
        .bind(must_change_password as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db) if db.message().contains("UNIQUE") => {
                AppError::Conflict(format!("user {username} already exists"))
            }
            other => AppError::Db(other),
        })?;
        Ok(result.last_insert_rowid())
    }
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct UserRow {
    pub id: i64,
    pub username: String,
    #[serde(skip)]
    pub password_hash: String,
    pub role: String,
    pub traffic_quota_bytes: Option<i64>,
    pub bandwidth_limit_bps: Option<i64>,
    pub must_change_password: i64,
    pub created_at: DateTime<Utc>,
}

impl UserRow {
    pub fn role(&self) -> Role {
        Role::from_str(&self.role).unwrap_or(Role::User)
    }
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct NodeRow {
    pub id: i64,
    pub name: String,
    #[serde(skip)]
    pub secret: String,
    pub api_endpoint: String,
    pub tunnel_endpoint: String,
    /// Transport protocol tunnel-clients use to reach this node
    /// (tcp | quic | websocket | kcp | wss). See ARCHITECTURE.md section 7.
    pub transport: String,
    /// gostc-style presentation fields.
    pub remark: String,
    /// Feature switches: 1 = enabled, 2 = disabled (gostc convention).
    pub web: i64,
    pub forward: i64,
    pub p2p: i64,
    /// Domain resolution config (gostc "域名解析" tab).
    pub http_port: String,
    pub domain: String,
    /// Forward port quota (gostc "端口配额", e.g. "10001-11000,20000").
    pub forward_ports: String,
    pub input_bytes: i64,
    pub output_bytes: i64,
    pub status: String,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TunnelRow {
    pub id: i64,
    pub user_id: i64,
    pub node_id: Option<i64>,
    pub client_id: Option<i64>,
    pub name: String,
    pub r#type: String,
    pub local_addr: String,
    pub remote_port: Option<i64>,
    pub domain: Option<String>,
    #[serde(skip)]
    pub token: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ClientRow {
    pub id: i64,
    pub user_id: i64,
    /// Kept for schema compatibility; clients are no longer bound to a node
    /// at creation time (the binding lives on tunnels now).
    #[allow(dead_code)]
    pub node_id: Option<i64>,
    pub name: String,
    pub token: String,
    pub status: String,
    pub last_online: Option<DateTime<Utc>>,
    pub input_bytes: i64,
    pub output_bytes: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct NoticeRow {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
    pub must_change_password: bool,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}