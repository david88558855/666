//! P2P session rendezvous (self-built; NAT-traversal principles referenced
//! from EasyTier: STUN probing, UDP hole punching, relay fallback).
//!
//! The console is the coordination plane for gostc-style P2P tunnels: the
//! service-side client (`tunnels.client_id`) and each visitor-side client
//! (`tunnel_visitors`) periodically register their STUN-probed public UDP
//! endpoint here and receive the peer snapshot back in the same round trip.
//! Actual UDP hole punching and the direct data plane (yamux + orbien
//! frames) land in Phase 4 (ARCHITECTURE.md 7.8); until then the node relay
//! path remains the active fallback and this rendezvous state is kept in
//! memory only — a console restart simply makes clients re-register.

use crate::db::{ClientRow, TunnelRow};
use crate::error::{AppError, AppResult};
use crate::extractors::AppState;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

/// Registrations older than this are pruned. Clients re-register on their
/// polling loop, so an expired entry simply means "that peer went away".
const SESSION_TTL: Duration = Duration::from_secs(600);

#[derive(Debug, Clone)]
struct Endpoint {
    client_id: i64,
    /// Public UDP mapping probed via STUN, "ip:port".
    endpoint: String,
    /// NAT behaviour probed via STUN: full-cone | restricted-cone |
    /// port-restricted | symmetric | unknown.
    nat: String,
    updated: Instant,
}

#[derive(Debug, Default)]
struct Session {
    service: Option<Endpoint>,
    visitors: HashMap<i64, Endpoint>,
}

static SESSIONS: LazyLock<Mutex<HashMap<i64, Session>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn prune(map: &mut HashMap<i64, Session>, now: Instant) {
    map.retain(|_, s| {
        if let Some(e) = s.service.as_ref() {
            if now.duration_since(e.updated) > SESSION_TTL {
                s.service = None;
            }
        }
        s.visitors
            .retain(|_, v| now.duration_since(v.updated) <= SESSION_TTL);
        s.service.is_some() || !s.visitors.is_empty()
    });
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    /// Client token (same credential as /clients/connect).
    pub token: String,
    pub tunnel_id: i64,
    /// STUN-probed public UDP endpoint of this client, "ip:port".
    pub endpoint: String,
    /// NAT behaviour probed via STUN (optional, defaults to unknown).
    #[serde(default)]
    pub nat: String,
}

/// Register this client's endpoint for a p2p tunnel and get the peer
/// snapshot back. Role is derived server-side: the client that owns the
/// tunnel is the service side; clients bound in `tunnel_visitors` are
/// visitors; anyone else is rejected (the panel-side vKey trust model —
/// the panel owner attaches visitors, so no raw sk is needed on this API).
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if req.token.is_empty() {
        return Err(AppError::Validation("token is empty".into()));
    }
    if req.endpoint.is_empty() {
        return Err(AppError::Validation("endpoint is empty".into()));
    }
    let client: ClientRow = sqlx::query_as::<_, ClientRow>("SELECT * FROM clients WHERE token = ?")
        .bind(&req.token)
        .fetch_optional(&state.db.pool)
        .await?
        .ok_or(AppError::Unauthorized)?;
    if client.status != "active" {
        return Err(AppError::Forbidden);
    }
    let tunnel: TunnelRow = sqlx::query_as::<_, TunnelRow>("SELECT * FROM tunnels WHERE id = ?")
        .bind(req.tunnel_id)
        .fetch_optional(&state.db.pool)
        .await?
        .ok_or(AppError::NotFound(format!("tunnel {}", req.tunnel_id)))?;
    if tunnel.r#type != "p2p" {
        return Err(AppError::Validation(
            "rendezvous is only available for p2p tunnels".into(),
        ));
    }

    let role = if tunnel.client_id == Some(client.id) {
        "service"
    } else {
        let n: (i64,) = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM tunnel_visitors WHERE tunnel_id = ? AND client_id = ?",
        )
        .bind(tunnel.id)
        .bind(client.id)
        .fetch_one(&state.db.pool)
        .await?;
        if n.0 == 0 {
            return Err(AppError::Forbidden);
        }
        "visitor"
    };

    let now = Instant::now();
    let mut map = SESSIONS.lock().unwrap(); // no await while the lock is held
    prune(&mut map, now);
    let session = map.entry(tunnel.id).or_default();
    let ep = Endpoint {
        client_id: client.id,
        endpoint: req.endpoint.clone(),
        nat: if req.nat.is_empty() {
            "unknown".into()
        } else {
            req.nat.clone()
        },
        updated: now,
    };
    if role == "service" {
        session.service = Some(ep);
    } else {
        session.visitors.insert(client.id, ep);
    }
    let resp = json!({
        "role": role,
        "tunnel_id": tunnel.id,
        "local_addr": tunnel.local_addr,
        "service": session.service.as_ref().map(|e| json!({
            "client_id": e.client_id,
            "endpoint": e.endpoint,
            "nat": e.nat,
        })),
        "visitors": session
            .visitors
            .values()
            .map(|e| json!({
                "client_id": e.client_id,
                "endpoint": e.endpoint,
                "nat": e.nat,
            }))
            .collect::<Vec<_>>(),
    });
    drop(map);
    Ok(Json(resp))
}
