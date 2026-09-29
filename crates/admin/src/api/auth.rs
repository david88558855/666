use crate::auth::{hash_password, issue_token, verify_password};
use crate::db::{ChangePasswordRequest, LoginRequest, LoginResponse, UserRow};
use crate::error::{AppError, AppResult};
use crate::extractors::{AppState, AuthUser};
use axum::extract::State;
use axum::Json;

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    if req.username.is_empty() || req.password.is_empty() {
        return Err(AppError::Validation("username and password are empty".into()));
    }
    let row: Option<UserRow> =
        sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE username = ?")
            .bind(&req.username)
            .fetch_optional(&state.db.pool)
            .await?;
    let Some(user) = row else {
        // Constant-time path: hash a dummy password to mitigate timing leaks.
        let _ = verify_password(&req.password, "$argon2id$v=19$m=19456,t=2,p=1$YWFhYWFhYWFhYWFhYWFhYQ$0000000000000000000000000000000000000000000");
        return Err(AppError::Unauthorized);
    };
    let ok = verify_password(&req.password, &user.password_hash)?;
    if !ok {
        return Err(AppError::Unauthorized);
    }
    let role = user.role();
    let (token, exp) = issue_token(
        &state.config.auth.jwt_secret,
        user.id,
        &user.username,
        role,
        state.config.auth.access_token_ttl_secs,
    )?;
    Ok(Json(LoginResponse {
        access_token: token,
        token_type: "Bearer",
        expires_in: exp - chrono::Utc::now().timestamp(),
        must_change_password: user.must_change_password != 0,
    }))
}

pub async fn change_password(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<ChangePasswordRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if req.new_password.len() < 8 {
        return Err(AppError::Validation(
            "new_password must be at least 8 characters".into(),
        ));
    }
    let row: UserRow = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = ?")
        .bind(user.id)
        .fetch_one(&state.db.pool)
        .await?;
    if !verify_password(&req.old_password, &row.password_hash)? {
        return Err(AppError::Validation("old_password is incorrect".into()));
    }
    let new_hash = hash_password(&req.new_password)?;
    sqlx::query("UPDATE users SET password_hash = ?, must_change_password = 0 WHERE id = ?")
        .bind(&new_hash)
        .bind(user.id)
        .execute(&state.db.pool)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}