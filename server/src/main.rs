use std::net::SocketAddr;
use std::path::PathBuf;

use server::state::AppState;
use server::{config, db, routes};
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config_path = std::env::var("CONFIG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("config.yaml"));

    let config = config::load(&config_path).unwrap_or_else(|error| {
        panic!("failed to load config from {config_path:?}: {error}");
    });

    let pool = db::connect_and_migrate(&config.database_url, &config.migrations_path)
        .await
        .unwrap_or_else(|error| panic!("failed to prepare database: {error}"));

    let host = config.host.clone();
    let port = config.port;
    let state = AppState::new(pool, config);
    let app = routes::router(state).layer(TraceLayer::new_for_http());

    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .unwrap_or_else(|error| panic!("invalid host/port {host}:{port}: {error}"));

    tracing::info!(%addr, "starting server");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|error| panic!("failed to bind {addr}: {error}"));

    axum::serve(listener, app)
        .await
        .unwrap_or_else(|error| panic!("server error: {error}"));
}
