//! Embedded web control panel served at `/`.
//!
//! The whole panel is a single self-contained HTML file (inline CSS/JS, no
//! external CDN) compiled into the binary via `include_str!`, so the admin
//! binary works on air-gapped intranets without any static file deployment.

use axum::response::Html;
use axum::routing::get;
use axum::Router;

const INDEX_HTML: &str = include_str!("index.html");

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

pub fn router() -> Router<crate::extractors::AppState> {
    Router::new()
        .route("/", get(index))
        .route("/index.html", get(index))
}
