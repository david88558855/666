use crate::db::ClientRow;
use crate::error::{AppError, AppResult};
use crate::extractors::{AdminUser, AppState};
use axum::extract::{Path, State};
use axum::Json;
use rand::RngCore;
use serde::Deserialize;
use serde_json::json;

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

pub async fn list(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<ClientRow>>> {
    let rows: Vec<ClientRow> = sqlx::query_as::<_, ClientRow>("SELECT * FROM clients ORDER BY id")
        .fetch_all(&state.db.pool)
        .await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct CreateClientRequest {
    pub name: String,
    pub node_id: i64,
}

pub async fn create(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(req): Json<CreateClientRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if req.name.is_empty() {
        return Err(AppError::Validation("name is empty".into()));
    }
    let node: Option<NodeRow> = sqlx::query_as::<_, NodeRow>("SELECT * FROM nodes WHERE id = ?")
        .bind(req.node_id)
        .fetch_optional(&state.db.pool)
        .await?;
    node.as_ref().ok_or_else(|| AppError::NotFound(format!("node {}", req.node_id)))?;

    let token = random_token();
    let result = sqlx::query(
        "INSERT INTO clients (user_id, node_id, name, token) VALUES (?, ?, ?, ?)",
    )
    .bind(admin.0.id)
    .bind(req.node_id)
    .bind(&req.name)
    .bind(&token)
    .execute(&state.db.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db) if db.message().contains("UNIQUE") => {
            AppError::Conflict(format!("client {} already exists", req.name))
        }
        other => AppError::Db(other),
    })?;
    Ok(Json(json!({
        "id": result.last_insert_rowid(),
        "name": req.name,
        "node_id": req.node_id,
        "token": token,
    })))
}

use crate::db::NodeRow;

pub async fn delete(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    // App-level cascade (SQLite ALTER TABLE cannot attach an FK).
    sqlx::query("DELETE FROM tunnels WHERE client_id = ?")
        .bind(id)
        .execute(&state.db.pool)
        .await?;
    let result = sqlx::query("DELETE FROM clients WHERE id = ?")
        .bind(id)
        .execute(&state.db.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("client {id}")));
    }
    Ok(Json(json!({ "ok": true })))
}
