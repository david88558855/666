use crate::db::NodeRow;
use crate::error::{AppError, AppResult};
use crate::extractors::AdminUser;
use crate::extractors::AppState;
use crate::models::NodeStatus;
use axum::extract::{Path, State};
use axum::Json;
use rand::RngCore;
use serde::Deserialize;
use serde_json::json;

fn random_secret() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    use std::fmt::Write;
    let mut s = String::with_capacity(64);
    for b in bytes {
        write!(&mut s, "{b:02x}").unwrap();
    }
    s
}

pub async fn list(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<NodeRow>>> {
    let rows: Vec<NodeRow> = sqlx::query_as("SELECT * FROM nodes ORDER BY id")
        .fetch_all(&state.db.pool)
        .await?;
    Ok(Json(rows))
}

pub async fn get(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<i64>,
) -> AppResult<Json<NodeRow>> {
    let row: Option<NodeRow> = sqlx::query_as("SELECT * FROM nodes WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db.pool)
        .await?;
    row.map(Json).ok_or(AppError::NotFound(format!("node {id}")))
}

#[derive(Deserialize)]
pub struct CreateNodeRequest {
    pub name: String,
    pub api_endpoint: String,
    pub tunnel_endpoint: String,
}

pub async fn create(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(req): Json<CreateNodeRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if req.name.is_empty() {
        return Err(AppError::Validation("name is empty".into()));
    }
    let secret = random_secret();
    let result = sqlx::query(
        "INSERT INTO nodes (name, secret, api_endpoint, tunnel_endpoint)
         VALUES (?, ?, ?, ?)",
    )
    .bind(&req.name)
    .bind(&secret)
    .bind(&req.api_endpoint)
    .bind(&req.tunnel_endpoint)
    .execute(&state.db.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db) if db.message().contains("UNIQUE") => {
            AppError::Conflict(format!("node {} already exists", req.name))
        }
        other => AppError::Db(other),
    })?;
    Ok(Json(json!({
        "id": result.last_insert_rowid(),
        "name": req.name,
        "secret": secret,
    })))
}

#[derive(Deserialize)]
pub struct UpdateNodeRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub api_endpoint: Option<String>,
    #[serde(default)]
    pub tunnel_endpoint: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

pub async fn update(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateNodeRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if let Some(name) = req.name {
        sqlx::query("UPDATE nodes SET name = ? WHERE id = ?")
            .bind(name)
            .bind(id)
            .execute(&state.db.pool)
            .await?;
    }
    if let Some(ep) = req.api_endpoint {
        sqlx::query("UPDATE nodes SET api_endpoint = ? WHERE id = ?")
            .bind(ep)
            .bind(id)
            .execute(&state.db.pool)
            .await?;
    }
    if let Some(ep) = req.tunnel_endpoint {
        sqlx::query("UPDATE nodes SET tunnel_endpoint = ? WHERE id = ?")
            .bind(ep)
            .bind(id)
            .execute(&state.db.pool)
            .await?;
    }
    if let Some(s) = req.status {
        let st = NodeStatus::from_str(&s)
            .ok_or_else(|| AppError::Validation(format!("invalid status: {s}")))?;
        sqlx::query("UPDATE nodes SET status = ? WHERE id = ?")
            .bind(st.as_str())
            .bind(id)
            .execute(&state.db.pool)
            .await?;
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM nodes WHERE id = ?")
        .bind(id)
        .execute(&state.db.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("node {id}")));
    }
    Ok(Json(json!({ "ok": true })))
}