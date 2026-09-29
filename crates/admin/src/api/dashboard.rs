use crate::db::NodeRow;
use crate::extractors::{AdminUser, AppState};
use crate::error::AppResult;
use axum::extract::State;
use axum::Json;
use chrono::{DateTime, Utc};
use serde_json::json;

/// Aggregate dashboard data, mirroring gostc-open's admin dashboard:
/// 11 count cards (minus the commercial check-in card) plus traffic
/// ranking lists. Traffic counters are wired but stay zero until the
/// orbien data plane lands (ARCHITECTURE.md section 7.6).
pub async fn count(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<serde_json::Value>> {
    let pool = &state.db.pool;

    let user: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    let nodes: Vec<NodeRow> =
        sqlx::query_as::<_, NodeRow>("SELECT * FROM nodes ORDER BY id")
            .fetch_all(pool)
            .await?;
    let now = Utc::now();
    let node_online = nodes
        .iter()
        .filter(|n| is_recent(n.last_heartbeat, now))
        .count() as i64;

    let client_rows: Vec<(Option<DateTime<Utc>>,)> =
        sqlx::query_as("SELECT last_online FROM clients")
            .fetch_all(pool)
            .await?;
    let client = client_rows.len() as i64;
    let client_online = client_rows
        .iter()
        .filter(|(t,)| is_recent(*t, now))
        .count() as i64;

    // Tunnel counts by presentation page: tcp -> 私有隧道, udp -> 端口转发,
    // http/https -> 域名解析. proxy/p2p have no data plane yet.
    let mut host = 0i64;
    let mut forward = 0i64;
    let mut tunnel = 0i64;
    let rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT type, COUNT(*) FROM tunnels GROUP BY type")
            .fetch_all(pool)
            .await?;
    for (t, n) in rows {
        match t.as_str() {
            "http" | "https" => host += n,
            "udp" => forward += n,
            "tcp" => tunnel += n,
            _ => {}
        }
    }

    // Today's traffic (UTC day) from traffic_logs; zero until observed.
    let (today_in, today_out): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(bytes_in), 0), COALESCE(SUM(bytes_out), 0)
         FROM traffic_logs WHERE recorded_at >= ?",
    )
    .bind(Utc::now().date_naive().to_string())
    .fetch_one(pool)
    .await?;

    // Rankings: nodes / clients by total traffic (real columns, zero now);
    // tunnel rankings come from traffic_logs aggregation (empty until obs).
    let node_obs: Vec<serde_json::Value> = {
        let mut v: Vec<&NodeRow> = nodes.iter().collect();
        v.sort_by_key(|n| n.input_bytes + n.output_bytes);
        v.reverse();
        v.truncate(20);
        v.iter()
            .map(|n| {
                json!({
                    "name": n.name,
                    "online": is_recent(n.last_heartbeat, now) as i64,
                    "inputBytes": n.input_bytes,
                    "outputBytes": n.output_bytes,
                })
            })
            .collect()
    };
    let client_obs: Vec<serde_json::Value> = {
        let mut rows: Vec<(String, Option<DateTime<Utc>>, i64, i64)> = sqlx::query_as(
            "SELECT name, last_online, input_bytes, output_bytes FROM clients",
        )
        .fetch_all(pool)
        .await?;
        rows.sort_by_key(|(_, _, i, o)| i + o);
        rows.reverse();
        rows.truncate(20);
        rows.iter()
            .map(|(name, t, i, o)| {
                json!({
                    "name": name,
                    "online": is_recent(*t, now) as i64,
                    "inputBytes": i,
                    "outputBytes": o,
                })
            })
            .collect()
    };

    Ok(Json(json!({
        "count": {
            "user": user,
            "node": nodes.len() as i64,
            "nodeOnline": node_online,
            "client": client,
            "clientOnline": client_online,
            "host": host,
            "forward": forward,
            "tunnel": tunnel,
            "proxy": 0,
            "p2p": 0,
            "inputBytes": today_in,
            "outputBytes": today_out,
        },
        "nodeObsDate": node_obs,
        "clientObsDate": client_obs,
        "hostObsDate": [],
        "forwardObsDate": [],
        "tunnelObsDate": [],
    })))
}

fn is_recent(t: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    match t {
        Some(t) => (now - t).num_seconds() < 90,
        None => false,
    }
}
