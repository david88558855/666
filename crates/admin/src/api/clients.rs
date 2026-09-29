use crate::db::{ClientRow, NodeRow};
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
}

/// Create a client credential. No node binding needed: clients connect to
/// the central console and are routed to nodes per-tunnel.
pub async fn create(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(req): Json<CreateClientRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if req.name.is_empty() {
        return Err(AppError::Validation("name is empty".into()));
    }
    let token = random_token();
    let result =
        sqlx::query("INSERT INTO clients (user_id, node_id, name, token) VALUES (?, NULL, ?, ?)")
            .bind(admin.0.id)
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
        "token": token,
    })))
}

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

#[derive(Deserialize)]
pub struct ConnectRequest {
    pub token: String,
}

/// Tunnel-client bootstrapping. Authenticated by client token, so the client
/// only needs the console address -- no node address, no startup order.
/// Returns the tunnel endpoints (per node) the client should open its
/// control channels to, derived from its active tunnels.
pub async fn connect(
    State(state): State<AppState>,
    Json(req): Json<ConnectRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if req.token.is_empty() {
        return Err(AppError::Validation("token is empty".into()));
    }
    let row: Option<ClientRow> =
        sqlx::query_as::<_, ClientRow>("SELECT * FROM clients WHERE token = ?")
            .bind(&req.token)
            .fetch_optional(&state.db.pool)
            .await?;
    let Some(client) = row else {
        return Err(AppError::Unauthorized);
    };
    if client.status != "active" {
        return Err(AppError::Forbidden);
    }
    sqlx::query("UPDATE clients SET last_online = ? WHERE id = ?")
        .bind(chrono::Utc::now())
        .bind(client.id)
        .execute(&state.db.pool)
        .await?;

    let nodes: Vec<NodeRow> = sqlx::query_as::<_, NodeRow>(
        "SELECT DISTINCT nodes.* FROM nodes
         INNER JOIN tunnels ON tunnels.node_id = nodes.id
         WHERE tunnels.client_id = ? AND tunnels.status = 'active'
         ORDER BY nodes.id",
    )
    .bind(client.id)
    .fetch_all(&state.db.pool)
    .await?;

    let node_entries: Vec<serde_json::Value> = nodes
        .iter()
        .map(|n| {
            json!({
                "id": n.id,
                "name": n.name,
                "tunnel_endpoint": n.tunnel_endpoint,
                "transport": n.transport,
            })
        })
        .collect();

    Ok(Json(json!({
        "client_id": client.id,
        "name": client.name,
        "nodes": node_entries,
    })))
}
