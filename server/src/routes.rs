//! HTTP route wiring.

pub mod health;

use axum::Router;
use axum::routing::get;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health::get_health))
        .with_state(state)
}
