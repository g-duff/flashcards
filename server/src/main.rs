mod config;
mod core;
mod http;
mod model;
mod store;

use config::{Config, LogFormat};
use http::AppState;
use store::Store;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let config = Config::from_env();
    init_tracing(&config.log_format);

    // In-memory card store, seeded with a few sample cards. This app has
    // no persistence yet — a restart resets the deck. Swap `Store` for a
    // SQLite-backed one (see the Sandy Bank downloads app) when it needs
    // to survive a redeploy.
    let state = AppState {
        store: Store::seeded(),
    };

    let app = http::router(state);

    // nginx routes /flashcards/api/ here and strips the prefix, so this
    // binary keeps its own routes (/healthz, /cards, ...). It must be
    // reachable ONLY via nginx — bind loopback (BIND_ADDR in the
    // .service file).
    tracing::info!(bind = %config.bind_addr, "starting server");
    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .expect("failed to bind to BIND_ADDR");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install ctrl_c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}

fn init_tracing(format: &LogFormat) {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    match format {
        LogFormat::Text => tracing_subscriber::fmt().with_env_filter(filter).init(),
        LogFormat::Json => tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init(),
    }
}
