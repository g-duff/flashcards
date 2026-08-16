//! SQL persistence for Categories. Pure imperative shell: all I/O, no
//! business logic.

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use thiserror::Error;

use super::{Category, CategoryId, CategoryName, CategorySort};

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("a category with this name already exists")]
    DuplicateName,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

fn map_write_error(source: sqlx::Error) -> RepositoryError {
    match &source {
        sqlx::Error::Database(db_error) if db_error.is_unique_violation() => {
            RepositoryError::DuplicateName
        }
        _ => RepositoryError::Database(source),
    }
}

/// Inserts a new Category and returns the persisted row.
pub async fn insert(
    pool: &SqlitePool,
    name: &CategoryName,
    now: DateTime<Utc>,
) -> Result<Category, RepositoryError> {
    let row = sqlx::query_as::<_, CategoryRow>(
        "INSERT INTO categories (name, normalized_name, created_at, updated_at)
         VALUES (?, ?, ?, ?)
         RETURNING id, name, created_at, updated_at",
    )
    .bind(&name.display)
    .bind(&name.normalized)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await
    .map_err(map_write_error)?;

    Ok(row.into())
}

/// Lists every Category, ordered by creation date or alphabetically by
/// display name (spec.md story 9).
pub async fn list(pool: &SqlitePool, sort: CategorySort) -> Result<Vec<Category>, RepositoryError> {
    let query = match sort {
        CategorySort::CreatedAt => {
            "SELECT id, name, created_at, updated_at FROM categories ORDER BY created_at ASC, id ASC"
        }
        CategorySort::Alphabetical => {
            "SELECT id, name, created_at, updated_at FROM categories ORDER BY normalized_name ASC, id ASC"
        }
    };

    let rows = sqlx::query_as::<_, CategoryRow>(query)
        .fetch_all(pool)
        .await?;

    Ok(rows.into_iter().map(Category::from).collect())
}

/// Finds a Category by durable ID, if one exists.
pub async fn find_by_id(
    pool: &SqlitePool,
    id: CategoryId,
) -> Result<Option<Category>, RepositoryError> {
    let row = sqlx::query_as::<_, CategoryRow>(
        "SELECT id, name, created_at, updated_at FROM categories WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(Category::from))
}

/// Renames a Category in place, preserving its durable ID. Returns `None`
/// if no Category with that ID exists.
pub async fn rename(
    pool: &SqlitePool,
    id: CategoryId,
    name: &CategoryName,
    now: DateTime<Utc>,
) -> Result<Option<Category>, RepositoryError> {
    let row = sqlx::query_as::<_, CategoryRow>(
        "UPDATE categories SET name = ?, normalized_name = ?, updated_at = ?
         WHERE id = ?
         RETURNING id, name, created_at, updated_at",
    )
    .bind(&name.display)
    .bind(&name.normalized)
    .bind(now)
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(map_write_error)?;

    Ok(row.map(Category::from))
}

/// Deletes a Category by durable ID. Returns whether a row was removed.
///
/// No orphan-membership check exists yet: Vocabulary Entries and Category
/// Memberships are introduced in ticket 05, at which point deletion must
/// reject `409` when it would remove the final membership of any entry
/// (grilled-spec.md sec. 2). Until then no membership can exist, so every
/// deletion is safe.
pub async fn delete(pool: &SqlitePool, id: CategoryId) -> Result<bool, RepositoryError> {
    let result = sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

#[derive(Debug, sqlx::FromRow)]
struct CategoryRow {
    id: CategoryId,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CategoryRow> for Category {
    fn from(row: CategoryRow) -> Self {
        Category {
            id: row.id,
            name: row.name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
