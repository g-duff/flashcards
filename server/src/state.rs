//! Shared application state injected into every axum handler.

use std::sync::Arc;

use sqlx::SqlitePool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState(Arc<AppStateInner>);

struct AppStateInner {
    pub db: SqlitePool,
    pub config: Config,
}

impl AppState {
    pub fn new(db: SqlitePool, config: Config) -> Self {
        Self(Arc::new(AppStateInner { db, config }))
    }

    pub fn db(&self) -> &SqlitePool {
        &self.0.db
    }

    pub fn config(&self) -> &Config {
        &self.0.config
    }
}
