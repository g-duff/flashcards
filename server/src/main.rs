mod config;
mod core;
mod http;
mod model;
mod store;

use std::path::Path;
use std::sync::Arc;

use config::{Config, LogFormat};
use core::{Leitner, Scheduler};
use http::AppState;
use store::Db;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let config = Config::from_env();
    init_tracing(&config.log_format);

    // SQLite-backed Term store. The file (and its parent directory) is
    // created on first run; the schema is brought up to date by the
    // embedded migrations. A fresh database is empty — there is no seed.
    let db = match Db::open(Path::new(&config.database_path)) {
        Ok(db) => db,
        Err(err) => {
            tracing::error!(error = %err, path = %config.database_path, "failed to open database");
            std::process::exit(1);
        }
    };
    tracing::info!(
        database_path = %config.database_path,
        pivot_lang = %config.pivot_lang,
        "configuration loaded"
    );

    // The scheduling seam: one strategy, resolved here (ADR-0001).
    // Swapping Leitner for SM-2/FSRS later is this line plus the new
    // `core` type — no schema change.
    let scheduler: Arc<dyn Scheduler + Send + Sync> = Arc::new(Leitner);

    let state = AppState { db, scheduler };
    let app = http::router(state);

    // nginx routes /flashcards/api/ here and strips the prefix, so this
    // binary keeps its own routes (/healthz, /terms, ...). It must be
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
