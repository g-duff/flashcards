//! SQL persistence for Vocabulary Entries and Category Memberships. Pure
//! imperative shell: all I/O, no business logic.

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use thiserror::Error;

use super::{VocabularyEntry, VocabularyEntryId};
use crate::categories::CategoryId;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("a vocabulary entry with this language-and-text pair already exists")]
    DuplicateIdentity,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

fn map_write_error(source: sqlx::Error) -> RepositoryError {
    match &source {
        sqlx::Error::Database(db_error) if db_error.is_unique_violation() => {
            RepositoryError::DuplicateIdentity
        }
        _ => RepositoryError::Database(source),
    }
}

pub struct NewEntry<'a> {
    pub source_language: &'a str,
    pub source_text: &'a str,
    pub target_language: &'a str,
    pub target_text: &'a str,
    pub normalized_identity: &'a str,
    pub category_ids: &'a [CategoryId],
}

async fn insert_entry_row(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    source_language: &str,
    source_text: &str,
    target_language: &str,
    target_text: &str,
    normalized_identity: &str,
    now: DateTime<Utc>,
) -> Result<EntryRow, sqlx::Error> {
    sqlx::query_as::<_, EntryRow>(
        "INSERT INTO vocabulary_entries
            (source_language, source_text, target_language, target_text, normalized_identity, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         RETURNING id, source_language, source_text, target_language, target_text, created_at, updated_at",
    )
    .bind(source_language)
    .bind(source_text)
    .bind(target_language)
    .bind(target_text)
    .bind(normalized_identity)
    .bind(now)
    .bind(now)
    .fetch_one(&mut **tx)
    .await
}

async fn insert_membership(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    category_id: CategoryId,
    entry_id: VocabularyEntryId,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO category_memberships (category_id, vocabulary_entry_id, created_at)
         VALUES (?, ?, ?)",
    )
    .bind(category_id)
    .bind(entry_id)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Inserts a new Vocabulary Entry and its Category Memberships in one
/// transaction, returning the persisted row (spec.md story 13, 16).
pub async fn insert(
    pool: &SqlitePool,
    entry: NewEntry<'_>,
    now: DateTime<Utc>,
) -> Result<VocabularyEntry, RepositoryError> {
    let mut tx = pool.begin().await?;

    let row = insert_entry_row(
        &mut tx,
        entry.source_language,
        entry.source_text,
        entry.target_language,
        entry.target_text,
        entry.normalized_identity,
        now,
    )
    .await
    .map_err(map_write_error)?;

    for category_id in entry.category_ids {
        insert_membership(&mut tx, *category_id, row.id, now).await?;
    }

    tx.commit().await?;

    Ok(VocabularyEntry {
        id: row.id,
        source_language: row.source_language,
        source_text: row.source_text,
        target_language: row.target_language,
        target_text: row.target_text,
        category_ids: entry.category_ids.to_vec(),
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub struct NewBulkEntry<'a> {
    pub source_language: &'a str,
    pub source_text: &'a str,
    pub target_language: &'a str,
    pub target_text: &'a str,
    pub normalized_identity: &'a str,
    pub category_id: CategoryId,
}

/// Inserts multiple Vocabulary Entries, each with its single resolved
/// Category Membership, in one transaction. A failure on any row (a
/// duplicate identity, in particular) rolls back the whole batch so no
/// partial rows persist (spec.md story 26; ticket 06).
pub async fn insert_bulk(
    pool: &SqlitePool,
    entries: &[NewBulkEntry<'_>],
    now: DateTime<Utc>,
) -> Result<Vec<VocabularyEntry>, RepositoryError> {
    let mut tx = pool.begin().await?;
    let mut inserted = Vec::with_capacity(entries.len());

    for entry in entries {
        let row = insert_entry_row(
            &mut tx,
            entry.source_language,
            entry.source_text,
            entry.target_language,
            entry.target_text,
            entry.normalized_identity,
            now,
        )
        .await
        .map_err(map_write_error)?;

        insert_membership(&mut tx, entry.category_id, row.id, now).await?;

        inserted.push(VocabularyEntry {
            id: row.id,
            source_language: row.source_language,
            source_text: row.source_text,
            target_language: row.target_language,
            target_text: row.target_text,
            category_ids: vec![entry.category_id],
            created_at: row.created_at,
            updated_at: row.updated_at,
        });
    }

    tx.commit().await?;

    Ok(inserted)
}

pub struct ListFilter {
    pub category_id: Option<CategoryId>,
    pub source_language: Option<String>,
    pub target_language: Option<String>,
}

/// Lists Vocabulary Entries, optionally filtered by Category or (unordered)
/// Language Pair (spec.md story 25 route table).
pub async fn list(
    pool: &SqlitePool,
    filter: ListFilter,
) -> Result<Vec<VocabularyEntry>, RepositoryError> {
    let rows = sqlx::query_as::<_, EntryRow>(
        "SELECT DISTINCT ve.id, ve.source_language, ve.source_text, ve.target_language, ve.target_text, ve.created_at, ve.updated_at
         FROM vocabulary_entries ve
         LEFT JOIN category_memberships cm ON cm.vocabulary_entry_id = ve.id
         WHERE (?1 IS NULL OR cm.category_id = ?1)
           AND (
             ?2 IS NULL OR ?3 IS NULL
             OR (ve.source_language = ?2 AND ve.target_language = ?3)
             OR (ve.source_language = ?3 AND ve.target_language = ?2)
           )
         ORDER BY ve.created_at ASC, ve.id ASC",
    )
    .bind(filter.category_id)
    .bind(&filter.source_language)
    .bind(&filter.target_language)
    .fetch_all(pool)
    .await?;

    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let category_ids = category_ids_for(pool, row.id).await?;
        entries.push(VocabularyEntry {
            id: row.id,
            source_language: row.source_language,
            source_text: row.source_text,
            target_language: row.target_language,
            target_text: row.target_text,
            category_ids,
            created_at: row.created_at,
            updated_at: row.updated_at,
        });
    }

    Ok(entries)
}

/// Finds a Vocabulary Entry by durable ID, if one exists.
pub async fn find_by_id(
    pool: &SqlitePool,
    id: VocabularyEntryId,
) -> Result<Option<VocabularyEntry>, RepositoryError> {
    let row = sqlx::query_as::<_, EntryRow>(
        "SELECT id, source_language, source_text, target_language, target_text, created_at, updated_at
         FROM vocabulary_entries WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let category_ids = category_ids_for(pool, row.id).await?;

    Ok(Some(VocabularyEntry {
        id: row.id,
        source_language: row.source_language,
        source_text: row.source_text,
        target_language: row.target_language,
        target_text: row.target_text,
        category_ids,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}

async fn category_ids_for(
    pool: &SqlitePool,
    entry_id: VocabularyEntryId,
) -> Result<Vec<CategoryId>, RepositoryError> {
    let ids = sqlx::query_scalar::<_, CategoryId>(
        "SELECT category_id FROM category_memberships WHERE vocabulary_entry_id = ? ORDER BY category_id ASC",
    )
    .bind(entry_id)
    .fetch_all(pool)
    .await?;

    Ok(ids)
}

pub struct EntryEdit<'a> {
    pub source_text: &'a str,
    pub target_text: &'a str,
    pub normalized_identity: &'a str,
    pub category_ids: &'a [CategoryId],
}

/// Updates a Vocabulary Entry's text and Category Memberships in one
/// transaction, preserving its durable ID and languages. Returns `None` if
/// no entry with that ID exists (spec.md story 19).
pub async fn update(
    pool: &SqlitePool,
    id: VocabularyEntryId,
    edit: EntryEdit<'_>,
    now: DateTime<Utc>,
) -> Result<Option<VocabularyEntry>, RepositoryError> {
    let mut tx = pool.begin().await?;

    let row = sqlx::query_as::<_, EntryRow>(
        "UPDATE vocabulary_entries
         SET source_text = ?, target_text = ?, normalized_identity = ?, updated_at = ?
         WHERE id = ?
         RETURNING id, source_language, source_text, target_language, target_text, created_at, updated_at",
    )
    .bind(edit.source_text)
    .bind(edit.target_text)
    .bind(edit.normalized_identity)
    .bind(now)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_write_error)?;

    let Some(row) = row else {
        tx.rollback().await?;
        return Ok(None);
    };

    sqlx::query("DELETE FROM category_memberships WHERE vocabulary_entry_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    for category_id in edit.category_ids {
        sqlx::query(
            "INSERT INTO category_memberships (category_id, vocabulary_entry_id, created_at)
             VALUES (?, ?, ?)",
        )
        .bind(category_id)
        .bind(id)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(Some(VocabularyEntry {
        id: row.id,
        source_language: row.source_language,
        source_text: row.source_text,
        target_language: row.target_language,
        target_text: row.target_text,
        category_ids: edit.category_ids.to_vec(),
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}

/// Deletes a Vocabulary Entry and its Category Memberships in one
/// transaction. Direction-specific Progress and practice history are not
/// yet introduced (tickets 07/08), so there is nothing further to clean up
/// today. Returns whether a row was removed (spec.md story 22, 23).
pub async fn delete(pool: &SqlitePool, id: VocabularyEntryId) -> Result<bool, RepositoryError> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM category_memberships WHERE vocabulary_entry_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    let result = sqlx::query("DELETE FROM vocabulary_entries WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(result.rows_affected() > 0)
}

/// Returns the Vocabulary Entry IDs, among `candidate_ids`, for which the
/// given Category is their *only* Category Membership. Used to reject
/// unsafe Category deletion (spec.md story 11, 12).
pub async fn entry_ids_with_only_membership(
    pool: &SqlitePool,
    category_id: CategoryId,
) -> Result<Vec<VocabularyEntryId>, RepositoryError> {
    let ids = sqlx::query_scalar::<_, VocabularyEntryId>(
        "SELECT vocabulary_entry_id FROM category_memberships
         WHERE category_id = ?
         GROUP BY vocabulary_entry_id
         HAVING (SELECT COUNT(*) FROM category_memberships cm2 WHERE cm2.vocabulary_entry_id = category_memberships.vocabulary_entry_id) = 1",
    )
    .bind(category_id)
    .fetch_all(pool)
    .await?;

    Ok(ids)
}

#[derive(Debug, sqlx::FromRow)]
struct EntryRow {
    id: VocabularyEntryId,
    source_language: String,
    source_text: String,
    target_language: String,
    target_text: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
