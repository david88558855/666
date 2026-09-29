//! Axum extractors that gate handlers behind JWT authentication.

use crate::auth::{parse_bearer, validate_token};
use crate::config::AppConfig;
use crate::db::Db;
use crate::error::AppError;
use crate::models::Role;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use std::str::FromStr;
use std::sync::Arc;

/// Shared application state injected into every handler.
#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub config: Arc<AppConfig>,
}

impl AppState {
    pub fn new(db: Db, config: AppConfig) -> Self {
        Self {
            db,
            config: Arc::new(config),
        }
    }
}

/// Authenticated user (any role).
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub username: String,
    pub role: Role,
}

#[async_trait::async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .ok_or(AppError::Unauthorized)?
            .to_str()
            .map_err(|_| AppError::Unauthorized)?;
        let token = parse_bearer(header).ok_or(AppError::Unauthorized)?;
        let claims = validate_token(&state.config.auth.jwt_secret, token)?;
        let id: i64 = claims.sub.parse().map_err(|_| AppError::Unauthorized)?;
        let role = Role::from_str(&claims.role).map_err(|_| AppError::Unauthorized)?;
        Ok(Self {
            id,
            username: claims.username,
            role,
        })
    }
}

/// Authenticated user with admin role.
#[derive(Debug, Clone)]
pub struct AdminUser(pub AuthUser);

#[async_trait::async_trait]
impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.role != Role::Admin {
            return Err(AppError::Forbidden);
        }
        Ok(Self(user))
    }
}