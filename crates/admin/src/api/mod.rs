pub mod auth;
pub mod clients;
pub mod dashboard;
pub mod health;
pub mod nodes;
pub mod notices;
pub mod p2p;
pub mod settings;
pub mod tunnels;
pub mod users;
pub mod visitors;

use crate::extractors::AppState;
use crate::web;
use axum::{routing::get, Router};

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(web::router())
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
        .route("/nodes/register", axum::routing::post(nodes::register))
        .route("/nodes/:id/secret", get(nodes::secret))
        .route("/clients", get(clients::list).post(clients::create))
        .route("/clients/connect", axum::routing::post(clients::connect))
        .route("/clients/:id", axum::routing::delete(clients::delete))
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
        .route("/dashboard", get(dashboard::count))
        .route("/notices", get(notices::list).post(notices::create))
        .route("/notices/:id", axum::routing::delete(notices::delete))
        .route("/settings", get(settings::get).put(settings::update))
        .route("/tunnels/:id/visitors", get(visitors::list).post(visitors::create))
        .route(
            "/tunnels/:id/visitors/:vid",
            axum::routing::delete(visitors::delete),
        )
        .route("/p2p/sessions", axum::routing::post(p2p::register))
        .with_state(state)
}