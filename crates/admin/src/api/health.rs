use crate::extractors::AppState;
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

pub async fn healthz(State(state): State<AppState>) -> Json<Value> {
    let db_ok = sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&state.db.pool)
        .await
        .is_ok();
    Json(json!({
        "status": if db_ok { "ok" } else { "degraded" },
        "service": "gostc-rs-admin",
        "db": db_ok,
    }))
}