use crate::auth::hash_password;
use crate::db::UserRow;
use crate::error::{AppError, AppResult};
use crate::extractors::{AdminUser, AppState, AuthUser};
use crate::models::Role;
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::str::FromStr;

pub async fn list(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<UserRow>>> {
    let rows: Vec<UserRow> = sqlx::query_as::<_, UserRow>("SELECT * FROM users ORDER BY id")
        .fetch_all(&state.db.pool)
        .await?;
    Ok(Json(rows))
}

pub async fn get(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> AppResult<Json<UserRow>> {
    if user.id != id && user.role != Role::Admin {
        return Err(AppError::Forbidden);
    }
    let row: Option<UserRow> = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db.pool)
        .await?;
    row.map(Json).ok_or(AppError::NotFound(format!("user {id}")))
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub role: String,
}

pub async fn create(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(req): Json<CreateUserRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if req.username.is_empty() {
        return Err(AppError::Validation("username is empty".into()));
    }
    if req.password.len() < 8 {
        return Err(AppError::Validation(
            "password must be at least 8 characters".into(),
        ));
    }
    let role = if req.role.is_empty() {
        Role::User
    } else {
        Role::from_str(&req.role).map_err(|_| AppError::Validation("invalid role".into()))?
    };
    let hash = hash_password(&req.password)?;
    let id = state.db.create_user(&req.username, &hash, role, true).await?;
    Ok(Json(json!({ "id": id, "username": req.username, "role": role })))
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub traffic_quota_bytes: Option<Option<i64>>,
    #[serde(default)]
    pub bandwidth_limit_bps: Option<Option<i64>>,
}

pub async fn update(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateUserRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if let Some(username) = req.username {
        let username = username.trim().to_string();
        if username.is_empty() {
            return Err(AppError::Validation("username is empty".into()));
        }
        sqlx::query("UPDATE users SET username = ? WHERE id = ?")
            .bind(&username)
            .bind(id)
            .execute(&state.db.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::Database(db) if db.message().contains("UNIQUE") => {
                    AppError::Conflict(format!("user {username} already exists"))
                }
                other => AppError::Db(other),
            })?;
    }
    if let Some(password) = req.password {
        if !password.is_empty() {
            if password.len() < 8 {
                return Err(AppError::Validation(
                    "password must be at least 8 characters".into(),
                ));
            }
            let hash = hash_password(&password)?;
            sqlx::query("UPDATE users SET password_hash = ?, must_change_password = 1 WHERE id = ?")
                .bind(hash)
                .bind(id)
                .execute(&state.db.pool)
                .await?;
        }
    }
    if let Some(role) = req.role {
        if id == admin.0.id {
            return Err(AppError::Validation("cannot modify your own role".into()));
        }
        let r = Role::from_str(&role).map_err(|_| AppError::Validation("invalid role".into()))?;
        sqlx::query("UPDATE users SET role = ? WHERE id = ?")
            .bind(r.as_str())
            .bind(id)
            .execute(&state.db.pool)
            .await?;
    }
    if let Some(q) = req.traffic_quota_bytes {
        sqlx::query("UPDATE users SET traffic_quota_bytes = ? WHERE id = ?")
            .bind(q)
            .bind(id)
            .execute(&state.db.pool)
            .await?;
    }
    if let Some(b) = req.bandwidth_limit_bps {
        sqlx::query("UPDATE users SET bandwidth_limit_bps = ? WHERE id = ?")
            .bind(b)
            .bind(id)
            .execute(&state.db.pool)
            .await?;
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    if id == admin.0.id {
        return Err(AppError::Validation("cannot delete yourself".into()));
    }
    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(&state.db.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("user {id}")));
    }
    Ok(Json(json!({ "ok": true })))
}