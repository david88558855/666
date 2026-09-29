use crate::db::{TunnelRow, VisitorRow};
use crate::error::{AppError, AppResult};
use crate::extractors::{AppState, AuthUser};
use crate::models::Role;
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

/// Visitor management for secret tunnels (stcp/sudp). A visitor binds a
/// visitor-side client and a local listen port to one secret tunnel; the
/// visitor client then opens that local port and forwards into the service
/// via P2P direct or node relay fallback (ARCHITECTURE.md 7.8/7.9).

async fn owned_tunnel(
    state: &AppState,
    user: &AuthUser,
    id: i64,
) -> AppResult<TunnelRow> {
    let row: Option<TunnelRow> =
        sqlx::query_as::<_, TunnelRow>("SELECT * FROM tunnels WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.db.pool)
            .await?;
    let row = row.ok_or(AppError::NotFound(format!("tunnel {id}")))?;
    if user.role != Role::Admin && row.user_id != user.id {
        return Err(AppError::Forbidden);
    }
    Ok(row)
}

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Vec<VisitorRow>>> {
    let tunnel = owned_tunnel(&state, &user, id).await?;
    let rows: Vec<VisitorRow> = sqlx::query_as::<_, VisitorRow>(
        "SELECT * FROM tunnel_visitors WHERE tunnel_id = ? ORDER BY id",
    )
    .bind(tunnel.id)
    .fetch_all(&state.db.pool)
    .await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct CreateVisitorRequest {
    pub client_id: i64,
    pub local_listen: i64,
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<CreateVisitorRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let tunnel = owned_tunnel(&state, &user, id).await?;
    if !matches!(tunnel.r#type.as_str(), "stcp" | "sudp") {
        return Err(AppError::Validation(
            "visitors can only be attached to stcp/sudp tunnels".into(),
        ));
    }
    if !(1..=65535).contains(&req.local_listen) {
        return Err(AppError::Validation(
            "local_listen must be a TCP/UDP port (1-65535)".into(),
        ));
    }
    // The visitor client must belong to the same owner as the tunnel
    // (admins may pick any client; the sk is shared inside the tunnel).
    let client: Option<crate::db::ClientRow> =
        sqlx::query_as::<_, crate::db::ClientRow>(
            "SELECT * FROM clients WHERE id = ?",
        )
        .bind(req.client_id)
        .fetch_optional(&state.db.pool)
        .await?;
    let Some(client) = client else {
        return Err(AppError::NotFound(format!("client {}", req.client_id)));
    };
    if user.role != Role::Admin && client.user_id != tunnel.user_id {
        return Err(AppError::Forbidden);
    }
    let dup: (i64,) = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM tunnel_visitors WHERE tunnel_id = ? AND client_id = ?",
    )
    .bind(tunnel.id)
    .bind(client.id)
    .fetch_one(&state.db.pool)
    .await?;
    if dup.0 > 0 {
        return Err(AppError::Conflict(
            "this client already visits this tunnel".into(),
        ));
    }
    let result = sqlx::query(
        "INSERT INTO tunnel_visitors (tunnel_id, client_id, local_listen)
         VALUES (?, ?, ?)",
    )
    .bind(tunnel.id)
    .bind(client.id)
    .bind(req.local_listen)
    .execute(&state.db.pool)
    .await?;
    Ok(Json(json!({
        "id": result.last_insert_rowid(),
        "tunnel_id": tunnel.id,
        "client_id": client.id,
        "local_listen": req.local_listen,
    })))
}

pub async fn delete(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, vid)): Path<(i64, i64)>,
) -> AppResult<Json<serde_json::Value>> {
    let tunnel = owned_tunnel(&state, &user, id).await?;
    let result = sqlx::query(
        "DELETE FROM tunnel_visitors WHERE id = ? AND tunnel_id = ?",
    )
    .bind(vid)
    .bind(tunnel.id)
    .execute(&state.db.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("visitor {vid}")));
    }
    Ok(Json(json!({ "ok": true })))
}
