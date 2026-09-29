use crate::extractors::{AdminUser, AppState};
use crate::error::{AppError, AppResult};
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

/// 系统配置-基础配置 (gostc "系统配置" page), key/value backed.
/// GET is public so the login page can render the site name.
pub async fn get(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT key, value FROM settings")
            .fetch_all(&state.db.pool)
            .await?;
    let map: serde_json::Map<String, serde_json::Value> = rows
        .into_iter()
        .map(|(k, v)| (k, json!(v)))
        .collect();
    Ok(Json(json!(map)))
}

#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    #[serde(default)]
    pub site_name: Option<String>,
}

pub async fn update(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(req): Json<UpdateSettingsRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if let Some(name) = &req.site_name {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::Validation("site_name is empty".into()));
        }
        if name.len() > 64 {
            return Err(AppError::Validation("site_name too long (max 64)".into()));
        }
        upsert(&state, "site_name", name).await?;
    }
    Ok(Json(json!({ "ok": true })))
}

async fn upsert(state: &AppState, key: &str, value: &str) -> AppResult<()> {
    sqlx::query("INSERT INTO settings (key, value) VALUES (?, ?)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(key)
        .bind(value)
        .execute(&state.db.pool)
        .await?;
    Ok(())
}
