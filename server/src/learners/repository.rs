//! SQL persistence for Learners. Pure imperative shell: all I/O, no
//! business logic.

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use thiserror::Error;

use super::{Learner, LearnerId, LearnerName};

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("a learner with this name already exists")]
    DuplicateName,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

fn map_insert_error(source: sqlx::Error) -> RepositoryError {
    match &source {
        sqlx::Error::Database(db_error) if db_error.is_unique_violation() => {
            RepositoryError::DuplicateName
        }
        _ => RepositoryError::Database(source),
    }
}

/// Inserts a new Learner and returns the persisted row.
pub async fn insert(
    pool: &SqlitePool,
    name: &LearnerName,
    now: DateTime<Utc>,
) -> Result<Learner, RepositoryError> {
    let row = sqlx::query_as::<_, LearnerRow>(
        "INSERT INTO learners (name, normalized_name, created_at, updated_at)
         VALUES (?, ?, ?, ?)
         RETURNING id, name, created_at, updated_at",
    )
    .bind(&name.display)
    .bind(&name.normalized)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(map_insert_error)?;

    Ok(row.into())
}

/// Lists every Learner ordered by creation time, oldest first.
pub async fn list(pool: &SqlitePool) -> Result<Vec<Learner>, RepositoryError> {
    let rows = sqlx::query_as::<_, LearnerRow>(
        "SELECT id, name, created_at, updated_at FROM learners ORDER BY created_at ASC, id ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Learner::from).collect())
}

/// Finds a Learner by durable ID, if one exists.
pub async fn find_by_id(
    pool: &SqlitePool,
    id: LearnerId,
) -> Result<Option<Learner>, RepositoryError> {
    let row = sqlx::query_as::<_, LearnerRow>(
        "SELECT id, name, created_at, updated_at FROM learners WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(Learner::from))
}

#[derive(Debug, sqlx::FromRow)]
struct LearnerRow {
    id: LearnerId,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<LearnerRow> for Learner {
    fn from(row: LearnerRow) -> Self {
        Learner {
            id: row.id,
            name: row.name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
