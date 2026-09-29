use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("internal: {0}")]
    Internal(#[from] anyhow::Error),

    #[error("database: {0}")]
    Db(#[from] sqlx::Error),

    #[error("migration: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("not found: {0}")]
    NotFound(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("validation: {0}")]
    Validation(String),

    #[error("password hashing: {0}")]
    PasswordHash(String),

    #[error("jwt: {0}")]
    Jwt(String),
}

impl AppError {
    fn status(&self) -> StatusCode {
        use AppError::*;
        match self {
            Unauthorized | Jwt(_) => StatusCode::UNAUTHORIZED,
            Forbidden => StatusCode::FORBIDDEN,
            NotFound(_) => StatusCode::NOT_FOUND,
            Conflict(_) => StatusCode::CONFLICT,
            Validation(_) | PasswordHash(_) => StatusCode::BAD_REQUEST,
            Internal(_) | Db(_) | Migrate(_) | Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        // Never leak internal details to clients but always log them.
        let body = match &self {
            AppError::Internal(_) | AppError::Db(_) | AppError::Migrate(_) | AppError::Io(_) => {
                tracing::error!(error = ?self, "internal error");
                json!({ "error": "internal_error" }).to_string()
            }
            other => json!({ "error": other.to_string() }).to_string(),
        };
        (status, Json(serde_json::from_str::<serde_json::Value>(&body).unwrap()))
            .into_response()
    }
}

pub type AppResult<T> = std::result::Result<T, AppError>;