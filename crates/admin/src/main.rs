mod api;
mod auth;
mod config;
mod db;
mod error;
mod extractors;
mod models;
mod web;

use anyhow::Context;
use config::AppConfig;
use db::Db;
use extractors::AppState;
use std::path::PathBuf;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter as TracingEnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let config_path = std::env::var("GOSTC_RS_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("config.toml"));
    let (config, config_created) = AppConfig::load_or_create(&config_path)
        .with_context(|| format!("loading config from {}", config_path.display()))?;
    if config_created {
        tracing::info!(path = %config_path.display(), "no config file found, generated a default one");
    }

    tracing::info!(
        bind_addr = %config.server.bind_addr,
        data_dir = %config.server.data_dir,
        "starting gostc-rs-admin"
    );

    std::fs::create_dir_all(&config.server.data_dir)
        .with_context(|| format!("create data dir {}", config.server.data_dir))?;

    let db = Db::connect(&config.database.url)
        .await
        .context("connect to database")?;
    db.migrate().await.context("run migrations")?;
    bootstrap_admin(&db, &config).await?;

    let state = AppState::new(db, config.clone());

    let app = api::router(state)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&config.server.bind_addr)
        .await
        .with_context(|| format!("bind {}", config.server.bind_addr))?;
    let port = config
        .server
        .bind_addr
        .rsplit(':')
        .next()
        .unwrap_or("8080")
        .to_string();
    if config_created {
        println!("============================================================");
        println!("  gostc-rs 管理服务已启动（首次运行，已自动生成配置文件）");
        println!("    配置文件    : {}", config_path.display());
        println!("    Web控制面板 : http://127.0.0.1:{port}/");
        println!("    管理员账号  : {}", config.bootstrap.admin_username);
        println!("    管理员密码  : {}", config.bootstrap.admin_password);
        println!("    ↑ 密码已写入配置文件，登录面板后请立即修改");
        println!("============================================================");
    } else {
        println!("gostc-rs admin 已启动: Web控制面板 http://127.0.0.1:{port}/");
    }
    tracing::info!("listening on {}", config.server.bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() {
    let filter = TracingEnvFilter::try_from_default_env()
        .unwrap_or_else(|_| TracingEnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}

async fn bootstrap_admin(db: &Db, config: &AppConfig) -> anyhow::Result<()> {
    use crate::auth::hash_password;
    use crate::models::Role;

    let count = db.user_count().await?;
    if count > 0 {
        return Ok(());
    }
    tracing::warn!(
        username = %config.bootstrap.admin_username,
        "no users yet, creating bootstrap admin from config (change password immediately)"
    );
    let hash = hash_password(&config.bootstrap.admin_password)?;
    db.create_user(
        &config.bootstrap.admin_username,
        &hash,
        Role::Admin,
        true,
    )
    .await?;
    Ok(())
}