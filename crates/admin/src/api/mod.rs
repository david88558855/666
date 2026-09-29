pub mod auth;
pub mod health;
pub mod nodes;
pub mod tunnels;
pub mod users;

use crate::extractors::AppState;
use axum::{routing::get, Router};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/auth/login", axum::routing::post(auth::login))
        .route("/auth/password", axum::routing::post(auth::change_password))
        .route("/users", get(users::list).post(users::create))
        .route(
            "/users/:id",
            get(users::get)
                .patch(users::update)
                .delete(users::delete),
        )
        .route("/nodes", get(nodes::list).post(nodes::create))
        .route(
            "/nodes/:id",
            get(nodes::get)
                .patch(nodes::update)
                .delete(nodes::delete),
        )
        .route("/tunnels", get(tunnels::list).post(tunnels::create))
        .route(
            "/tunnels/:id",
            get(tunnels::get)
                .patch(tunnels::update)
                .delete(tunnels::delete),
        )
        .with_state(state)
}