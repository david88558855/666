use crate::db::TunnelRow;
use crate::error::{AppError, AppResult};
use crate::extractors::{AdminUser, AppState, AuthUser};
use crate::models::{Role, TunnelStatus, TunnelType};
use axum::extract::{Path, State};
use axum::Json;
use rand::RngCore;
use serde::Deserialize;
use serde_json::json;
use std::str::FromStr;

fn random_token() -> String {
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    use std::fmt::Write;
    let mut s = String::with_capacity(48);
    for b in bytes {
        write!(&mut s, "{b:02x}").unwrap();
    }
    s
}

fn validate_tunnel(r#type: &TunnelType, remote_port: &Option<i64>, domain: &Option<String>) -> AppResult<()> {
    match r#type {
        TunnelType::Tcp | TunnelType::Udp => {
            if remote_port.is_none() {
                return Err(AppError::Validation(
                    "remote_port is required for tcp/udp tunnels".into(),
                ));
            }
        }
        TunnelType::Http | TunnelType::Https => {
            if domain.is_none() || domain.as_deref().unwrap_or("").is_empty() {
                return Err(AppError::Validation(
                    "domain is required for http/https tunnels".into(),
                ));
            }
        }
    }
    Ok(())
}

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<TunnelRow>>> {
    let rows: Vec<TunnelRow> = if user.role == Role::Admin {
        sqlx::query_as("SELECT * FROM tunnels ORDER BY id")
            .fetch_all(&state.db.pool)
            .await?
    } else {
        sqlx::query_as("SELECT * FROM tunnels WHERE user_id = ? ORDER BY id")
            .bind(user.id)
            .fetch_all(&state.db.pool)
            .await?
    };
    Ok(Json(rows))
}

pub async fn get(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> AppResult<Json<TunnelRow>> {
    let row: Option<TunnelRow> = sqlx::query_as("SELECT * FROM tunnels WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db.pool)
        .await?;
    let row = row.ok_or(AppError::NotFound(format!("tunnel {id}")))?;
    if user.role != Role::Admin && row.user_id != user.id {
        return Err(AppError::Forbidden);
    }
    Ok(Json(row))
}

#[derive(Deserialize)]
pub struct CreateTunnelRequest {
    pub node_id: i64,
    pub name: String,
    pub r#type: String,
    pub local_addr: String,
    #[serde(default)]
    pub remote_port: Option<i64>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateTunnelRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if req.name.is_empty() {
        return Err(AppError::Validation("name is empty".into()));
    }
    let r#type = TunnelType::from_str(&req.r#type)
        .map_err(|e| AppError::Validation(e))?;
    let status = match req.status.as_deref() {
        Some(s) => TunnelStatus::from_str(s)
            .ok_or_else(|| AppError::Validation(format!("invalid status: {s}")))?,
        None => TunnelStatus::Paused,
    };
    validate_tunnel(&r#type, &req.remote_port, &req.domain)?;
    let token = random_token();
    let result = sqlx::query(
        "INSERT INTO tunnels
           (user_id, node_id, name, type, local_addr, remote_port, domain, token, status)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(user.id)
    .bind(req.node_id)
    .bind(&req.name)
    .bind(r#type.as_str())
    .bind(&req.local_addr)
    .bind(req.remote_port)
    .bind(&req.domain)
    .bind(&token)
    .bind(status.as_str())
    .execute(&state.db.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db) if db.message().contains("UNIQUE") => AppError::Conflict(
            "a tunnel with this name already exists on that node".into(),
        ),
        other => AppError::Db(other),
    })?;
    Ok(Json(json!({
        "id": result.last_insert_rowid(),
        "name": req.name,
        "type": r#type,
        "token": token,
    })))
}

#[derive(Deserialize)]
pub struct UpdateTunnelRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub local_addr: Option<String>,
    #[serde(default)]
    pub remote_port: Option<Option<i64>>,
    #[serde(default)]
    pub domain: Option<Option<String>>,
    #[serde(default)]
    pub status: Option<String>,
}

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTunnelRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let row: TunnelRow = sqlx::query_as("SELECT * FROM tunnels WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db.pool)
        .await?
        .ok_or(AppError::NotFound(format!("tunnel {id}")))?;
    if user.role != Role::Admin && row.user_id != user.id {
        return Err(AppError::Forbidden);
    }
    if let Some(name) = req.name {
        sqlx::query("UPDATE tunnels SET name = ? WHERE id = ?")
            .bind(name)
            .bind(id)
            .execute(&state.db.pool)
            .await?;
    }
    if let Some(local_addr) = req.local_addr {
        sqlx::query("UPDATE tunnels SET local_addr = ? WHERE id = ?")
            .bind(local_addr)
            .bind(id)
            .execute(&state.db.pool)
            .await?;
    }
    if let Some(remote_port) = req.remote_port {
        sqlx::query("UPDATE tunnels SET remote_port = ? WHERE id = ?")
            .bind(remote_port)
            .bind(id)
            .execute(&state.db.pool)
            .await?;
    }
    if let Some(domain) = req.domain {
        sqlx::query("UPDATE tunnels SET domain = ? WHERE id = ?")
            .bind(domain)
            .bind(id)
            .execute(&state.db.pool)
            .await?;
    }
    if let Some(s) = req.status {
        let st = TunnelStatus::from_str(&s)
            .ok_or_else(|| AppError::Validation(format!("invalid status: {s}")))?;
        sqlx::query("UPDATE tunnels SET status = ? WHERE id = ?")
            .bind(st.as_str())
            .bind(id)
            .execute(&state.db.pool)
            .await?;
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let row: Option<TunnelRow> = sqlx::query_as("SELECT * FROM tunnels WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db.pool)
        .await?;
    let row = row.ok_or(AppError::NotFound(format!("tunnel {id}")))?;
    if user.role != Role::Admin && row.user_id != user.id {
        return Err(AppError::Forbidden);
    }
    sqlx::query("DELETE FROM tunnels WHERE id = ?")
        .bind(id)
        .execute(&state.db.pool)
        .await?;
    Ok(Json(json!({ "ok": true })))
}