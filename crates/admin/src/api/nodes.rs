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
use std::str::FromStr;

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
    let rows: Vec<NodeRow> = sqlx::query_as::<_, NodeRow>("SELECT * FROM nodes ORDER BY id")
        .fetch_all(&state.db.pool)
        .await?;
    Ok(Json(rows))
}

pub async fn get(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<i64>,
) -> AppResult<Json<NodeRow>> {
    let row: Option<NodeRow> = sqlx::query_as::<_, NodeRow>("SELECT * FROM nodes WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db.pool)
        .await?;
    row.map(Json).ok_or(AppError::NotFound(format!("node {id}")))
}

/// Transport protocols a node can fix for tunnel-clients (orbien-aligned).
pub const TRANSPORTS: [&str; 4] = ["tcp", "quic", "websocket", "kcp"];

fn validate_transport(t: &str) -> AppResult<()> {
    if TRANSPORTS.contains(&t) {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "invalid transport: {t} (expected one of {TRANSPORTS:?})"
        )))
    }
}

#[derive(Deserialize)]
pub struct CreateNodeRequest {
    pub name: String,
    pub api_endpoint: String,
    pub tunnel_endpoint: String,
    /// Transport protocol for tunnel-clients; defaults to tcp.
    #[serde(default)]
    pub transport: String,
}

pub async fn create(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(req): Json<CreateNodeRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if req.name.is_empty() {
        return Err(AppError::Validation("name is empty".into()));
    }
    let transport = if req.transport.is_empty() {
        "tcp".to_string()
    } else {
        validate_transport(&req.transport)?;
        req.transport.clone()
    };
    let secret = random_secret();
    let result = sqlx::query(
        "INSERT INTO nodes (name, secret, api_endpoint, tunnel_endpoint, transport)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&req.name)
    .bind(&secret)
    .bind(&req.api_endpoint)
    .bind(&req.tunnel_endpoint)
    .bind(&transport)
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
        "transport": transport,
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
    pub transport: Option<String>,
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
    if let Some(t) = req.transport {
        validate_transport(&t)?;
        sqlx::query("UPDATE nodes SET transport = ? WHERE id = ?")
            .bind(t)
            .bind(id)
            .execute(&state.db.pool)
            .await?;
    }
    if let Some(s) = req.status {
        let st = NodeStatus::from_str(&s)
            .map_err(|_| AppError::Validation(format!("invalid status: {s}")))?;
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

/// Joined row: a tunnel on this node together with its owning client.
#[derive(sqlx::FromRow)]
pub struct JoinedTunnel {
    c_id: i64,
    c_name: String,
    c_token: String,
    c_status: String,
    t_id: i64,
    t_name: String,
    t_type: String,
    t_local_addr: String,
    t_remote_port: Option<i64>,
    t_status: String,
}

#[derive(Deserialize)]
pub struct RegisterNodeRequest {
    pub secret: String,
}

/// Node self-registration / heartbeat. Authenticated by node secret.
/// Returns the node identity plus the clients and tunnel assignments bound
/// to this node, so the tunnel-server can open public listeners and
/// validate tunnel-clients.
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterNodeRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if req.secret.is_empty() {
        return Err(AppError::Validation("secret is empty".into()));
    }
    let row: Option<NodeRow> =
        sqlx::query_as::<_, NodeRow>("SELECT * FROM nodes WHERE secret = ?")
            .bind(&req.secret)
            .fetch_optional(&state.db.pool)
            .await?;
    let Some(node) = row else {
        return Err(AppError::Unauthorized);
    };
    if node.status == "disabled" {
        return Err(AppError::Forbidden);
    }
    sqlx::query("UPDATE nodes SET status = 'online', last_heartbeat = ? WHERE id = ?")
        .bind(chrono::Utc::now())
        .bind(node.id)
        .execute(&state.db.pool)
        .await?;

    let rows: Vec<JoinedTunnel> = sqlx::query_as::<_, JoinedTunnel>(
        "SELECT c.id AS c_id, c.name AS c_name, c.token AS c_token, c.status AS c_status,
                t.id AS t_id, t.name AS t_name, t.type AS t_type,
                t.local_addr AS t_local_addr, t.remote_port AS t_remote_port,
                t.status AS t_status
         FROM tunnels t
         JOIN clients c ON t.client_id = c.id
         WHERE t.node_id = ?
         ORDER BY c.id, t.id",
    )
    .bind(node.id)
    .fetch_all(&state.db.pool)
    .await?;

    // Group tunnels by client token.
    let mut grouped: std::collections::BTreeMap<i64, serde_json::Value> =
        std::collections::BTreeMap::new();
    for r in rows {
        let entry = grouped.entry(r.c_id).or_insert_with(|| {
            json!({
                "id": r.c_id,
                "name": r.c_name,
                "token": r.c_token,
                "status": r.c_status,
                "tunnels": Vec::<serde_json::Value>::new(),
            })
        });
        if let Some(tunnels) = entry.get_mut("tunnels").and_then(|t| t.as_array_mut()) {
            tunnels.push(json!({
                "id": r.t_id,
                "name": r.t_name,
                "type": r.t_type,
                "local_addr": r.t_local_addr,
                "remote_port": r.t_remote_port,
                "status": r.t_status,
            }));
        }
    }

    Ok(Json(json!({
        "node_id": node.id,
        "name": node.name,
        "tunnel_endpoint": node.tunnel_endpoint,
        "transport": node.transport,
        "clients": grouped.into_values().collect::<Vec<_>>(),
    })))
}

/// Reveal a node secret to admins (needed to re-show the server command).
pub async fn secret(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let row: Option<NodeRow> = sqlx::query_as::<_, NodeRow>("SELECT * FROM nodes WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db.pool)
        .await?;
    row.map(|n| Json(json!({ "secret": n.secret })))
        .ok_or(AppError::NotFound(format!("node {id}")))
}