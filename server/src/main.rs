use std::net::SocketAddr;
use std::path::PathBuf;

use server::state::AppState;
use server::{config, db, http, telemetry};

#[tokio::main]
async fn main() {
    let config_path = std::env::var("CONFIG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("config.yaml"));

    let config = config::load(&config_path).unwrap_or_else(|error| {
        panic!("failed to load config from {config_path:?}: {error}");
    });

    telemetry::init(&config.logging);

    tracing::info!(config_path = %config_path.display(), "config loaded");

    let pool = db::connect_and_migrate(&config.database_url, &config.migrations_path)
        .await
        .unwrap_or_else(|error| {
            tracing::error!(error = %error, "failed to prepare database");
            panic!("failed to prepare database: {error}");
        });

    let host = config.host.clone();
    let port = config.port;
    let state = AppState::new(pool, config);
    let app = telemetry::instrument_http(http::router(state));

    let addr: SocketAddr = format!("{host}:{port}").parse().unwrap_or_else(|error| {
        tracing::error!(error = %error, host = %host, port, "invalid host/port");
        panic!("invalid host/port {host}:{port}: {error}");
    });

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|error| {
            tracing::error!(error = %error, %addr, "failed to bind listener");
            panic!("failed to bind {addr}: {error}");
        });

    tracing::info!(%addr, "starting server");

    axum::serve(listener, app).await.unwrap_or_else(|error| {
        tracing::error!(error = %error, "server error");
        panic!("server error: {error}");
    });

    tracing::info!("server exiting");
}
