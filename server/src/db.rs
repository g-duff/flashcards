//! SQLite connection pool setup and migration runner. Pure imperative
//! shell: all I/O, no business logic.

use std::path::Path;

use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("failed to connect to database at {url}: {source}")]
    Connect {
        url: String,
        #[source]
        source: sqlx::Error,
    },
    #[error("failed to run migrations from {path}: {source}")]
    Migrate {
        path: String,
        #[source]
        source: sqlx::migrate::MigrateError,
    },
}

/// Opens a connection pool to `database_url`, creating the database file if
/// it does not already exist, then runs every pending migration found under
/// `migrations_path`.
pub async fn connect_and_migrate(
    database_url: &str,
    migrations_path: &str,
) -> Result<SqlitePool, DbError> {
    let options: SqliteConnectOptions =
        database_url.parse().map_err(|source| DbError::Connect {
            url: database_url.to_string(),
            source,
        })?;
    let options = options.create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .map_err(|source| DbError::Connect {
            url: database_url.to_string(),
            source,
        })?;

    let migrator = Migrator::new(Path::new(migrations_path))
        .await
        .map_err(|source| DbError::Migrate {
            path: migrations_path.to_string(),
            source,
        })?;

    migrator
        .run(&pool)
        .await
        .map_err(|source| DbError::Migrate {
            path: migrations_path.to_string(),
            source,
        })?;

    Ok(pool)
}
