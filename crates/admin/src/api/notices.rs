use crate::db::NoticeRow;
use crate::extractors::{AdminUser, AppState, AuthUser};
use crate::error::{AppError, AppResult};
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

/// 通知公告 (gostc "通知公告" page): admin publishes, any logged-in
/// account reads.
pub async fn list(
    State(state): State<AppState>,
    _user: AuthUser,
) -> AppResult<Json<Vec<NoticeRow>>> {
    let rows: Vec<NoticeRow> =
        sqlx::query_as::<_, NoticeRow>("SELECT * FROM notices ORDER BY id DESC")
            .fetch_all(&state.db.pool)
            .await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct CreateNoticeRequest {
    pub title: String,
    pub content: String,
}

pub async fn create(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(req): Json<CreateNoticeRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if req.title.trim().is_empty() {
        return Err(AppError::Validation("title is empty".into()));
    }
    if req.content.trim().is_empty() {
        return Err(AppError::Validation("content is empty".into()));
    }
    let result = sqlx::query("INSERT INTO notices (title, content) VALUES (?, ?)")
        .bind(req.title.trim())
        .bind(req.content.trim())
        .execute(&state.db.pool)
        .await?;
    Ok(Json(json!({ "id": result.last_insert_rowid() })))
}

pub async fn delete(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM notices WHERE id = ?")
        .bind(id)
        .execute(&state.db.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("notice {id}")));
    }
    Ok(Json(json!({ "ok": true })))
}
