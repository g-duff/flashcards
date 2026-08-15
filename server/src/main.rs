use std::net::SocketAddr;
use std::path::PathBuf;

use server::config::LogFormat;
use server::state::AppState;
use server::{config, db, http};
use tower_http::LatencyUnit;
use tower_http::request_id::{
    MakeRequestUuid, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Builds the per-request `INFO` span carrying `request_id` so every log
/// line emitted while handling the request (including handler-level
/// domain events) can be correlated to it. `request_id` is intentionally
/// distinct from OTel's `trace_id`/`span_id` (different field name,
/// different header, different ID format) so a future OTel layer can be
/// added without collision.
fn make_request_span<B>(request: &axum::http::Request<B>) -> tracing::Span {
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .and_then(|id| id.header_value().to_str().ok())
        .unwrap_or("unknown");

    tracing::info_span!(
        "request",
        method = %request.method(),
        uri = %request.uri(),
        request_id = %request_id,
    )
}

#[tokio::main]
async fn main() {
    let config_path = std::env::var("CONFIG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("config.yaml"));

    let config = config::load(&config_path).unwrap_or_else(|error| {
        panic!("failed to load config from {config_path:?}: {error}");
    });

    let filter =
        tracing_subscriber::EnvFilter::try_new(&config.logging.level).unwrap_or_else(|error| {
            panic!("invalid logging.level {:?}: {error}", config.logging.level)
        });
    let registry = tracing_subscriber::registry().with(filter);
    match config.logging.format {
        LogFormat::Json => registry
            .with(tracing_subscriber::fmt::layer().json())
            .init(),
        LogFormat::Text => registry.with(tracing_subscriber::fmt::layer()).init(),
    }

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
    let app = http::router(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(make_request_span)
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(LatencyUnit::Millis),
                ),
        )
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid));

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
