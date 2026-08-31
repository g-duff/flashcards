//! SQLite-backed Term store. The imperative shell's persistence: a single
//! `rusqlite` connection behind a mutex, every query run on a blocking
//! thread via [`tokio::task::spawn_blocking`] so it never stalls the
//! async runtime.
//!
//! The schema is created by an embedded `rusqlite_migration` migration
//! (version tracked in SQLite's `user_version`). There is no seed — a
//! fresh database is empty.

use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex};

use rusqlite::{Connection, params};
use rusqlite_migration::{M, Migrations};

use crate::model::{NewTerm, Term};

/// Embedded schema history. v1 creates the `term` table; later tickets
/// append `M::up(...)` entries (cards, reviews).
static MIGRATIONS: LazyLock<Migrations<'static>> =
    LazyLock::new(|| Migrations::new(vec![M::up(include_str!("../migrations/001_terms.sql"))]));

/// Everything that can go wrong bringing the database up at startup.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    #[error("creating the database directory: {0}")]
    Dir(#[source] std::io::Error),
    #[error("opening the database: {0}")]
    Open(#[from] rusqlite::Error),
    #[error("running migrations: {0}")]
    Migrate(#[from] rusqlite_migration::Error),
}

/// A handle to the Term store. Cheap to clone — every clone shares the
/// one underlying connection.
#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    /// Open (creating if absent) the database at `path`, ensuring its
    /// parent directory exists, enabling WAL and foreign-key enforcement,
    /// and migrating the schema to the latest version.
    pub fn open(path: &Path) -> Result<Self, OpenError> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(OpenError::Dir)?;
        }
        let mut conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
        MIGRATIONS.to_latest(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Run `f` against the connection on a blocking thread. The mutex is
    /// held only for the duration of `f`.
    async fn with<T, F>(&self, f: F) -> T
    where
        F: FnOnce(&mut Connection) -> T + Send + 'static,
        T: Send + 'static,
    {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let mut guard = conn.lock().expect("db mutex poisoned");
            f(&mut guard)
        })
        .await
        .expect("db blocking task panicked")
    }

    /// Every Term, oldest first.
    pub async fn list_terms(&self) -> rusqlite::Result<Vec<Term>> {
        self.with(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, foreign_lang, foreign_text, pivot_text, notes, created_at \
                 FROM term ORDER BY created_at, id",
            )?;
            let terms = stmt
                .query_map([], row_to_term)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(terms)
        })
        .await
    }

    /// Insert a Term under its precomputed `id`. Idempotent: a second
    /// insert of the same `id` is a no-op and the already-stored row is
    /// returned unchanged, so re-posting a Term never duplicates it.
    pub async fn insert_term(
        &self,
        id: String,
        new: NewTerm,
        created_at: String,
    ) -> rusqlite::Result<Term> {
        self.with(move |conn| {
            conn.execute(
                "INSERT INTO term (id, foreign_lang, foreign_text, pivot_text, notes, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(id) DO NOTHING",
                params![
                    id,
                    new.foreign_lang,
                    new.foreign_text,
                    new.pivot_text,
                    new.notes,
                    created_at,
                ],
            )?;
            fetch_term(conn, &id)
        })
        .await
    }

    /// Set a Term's `notes` (the only mutable column). Returns the updated
    /// Term, or `None` if no Term has that `id`.
    pub async fn patch_term_notes(
        &self,
        id: String,
        notes: Option<String>,
    ) -> rusqlite::Result<Option<Term>> {
        self.with(move |conn| {
            let affected = conn.execute(
                "UPDATE term SET notes = ?2 WHERE id = ?1",
                params![id, notes],
            )?;
            if affected == 0 {
                Ok(None)
            } else {
                fetch_term(conn, &id).map(Some)
            }
        })
        .await
    }

    /// Delete a Term. Returns whether a row actually went.
    pub async fn delete_term(&self, id: String) -> rusqlite::Result<bool> {
        self.with(move |conn| {
            conn.execute("DELETE FROM term WHERE id = ?1", [&id])
                .map(|n| n > 0)
        })
        .await
    }
}

fn fetch_term(conn: &Connection, id: &str) -> rusqlite::Result<Term> {
    conn.query_row(
        "SELECT id, foreign_lang, foreign_text, pivot_text, notes, created_at \
         FROM term WHERE id = ?1",
        [id],
        row_to_term,
    )
}

fn row_to_term(row: &rusqlite::Row<'_>) -> rusqlite::Result<Term> {
    Ok(Term {
        id: row.get(0)?,
        foreign_lang: row.get(1)?,
        foreign_text: row.get(2)?,
        pivot_text: row.get(3)?,
        notes: row.get(4)?,
        created_at: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_db() -> (TempDir, Db) {
        let dir = TempDir::new().expect("tempdir");
        // A nested path proves the parent dir is created on open.
        let db = Db::open(&dir.path().join("nested/flashcards.db")).expect("open db");
        (dir, db)
    }

    fn a_term() -> NewTerm {
        NewTerm {
            foreign_lang: "es".to_string(),
            foreign_text: "perro".to_string(),
            pivot_text: "dog".to_string(),
            notes: Some("el perro (m)".to_string()),
        }
    }

    #[tokio::test]
    async fn insert_then_list_roundtrips_the_term() {
        let (_dir, db) = temp_db();
        let stored = db
            .insert_term(
                "id-1".to_string(),
                a_term(),
                "2026-01-01T00:00:00Z".to_string(),
            )
            .await
            .expect("insert");

        assert_eq!(stored.foreign_text, "perro");
        assert_eq!(stored.notes.as_deref(), Some("el perro (m)"));
        assert_eq!(db.list_terms().await.expect("list"), vec![stored]);
    }

    #[tokio::test]
    async fn re_inserting_the_same_id_is_a_no_op() {
        let (_dir, db) = temp_db();
        let first = db
            .insert_term(
                "id-1".to_string(),
                a_term(),
                "2026-01-01T00:00:00Z".to_string(),
            )
            .await
            .expect("first insert");

        let second = db
            .insert_term(
                "id-1".to_string(),
                NewTerm {
                    notes: Some("a different note".to_string()),
                    ..a_term()
                },
                "2027-09-09T00:00:00Z".to_string(),
            )
            .await
            .expect("second insert");

        assert_eq!(first, second, "the originally stored row wins");
        assert_eq!(db.list_terms().await.expect("list").len(), 1);
    }

    #[tokio::test]
    async fn patch_notes_changes_only_notes_and_can_clear_them() {
        let (_dir, db) = temp_db();
        db.insert_term("id-1".to_string(), a_term(), "t".to_string())
            .await
            .expect("insert");

        let patched = db
            .patch_term_notes("id-1".to_string(), Some("el perro".to_string()))
            .await
            .expect("patch")
            .expect("some term");
        assert_eq!(patched.notes.as_deref(), Some("el perro"));
        assert_eq!(patched.foreign_text, "perro");

        let cleared = db
            .patch_term_notes("id-1".to_string(), None)
            .await
            .expect("patch")
            .expect("some term");
        assert_eq!(cleared.notes, None);
    }

    #[tokio::test]
    async fn patch_unknown_id_returns_none() {
        let (_dir, db) = temp_db();
        assert!(
            db.patch_term_notes("nope".to_string(), None)
                .await
                .expect("patch")
                .is_none()
        );
    }

    #[tokio::test]
    async fn delete_reports_whether_a_row_went() {
        let (_dir, db) = temp_db();
        db.insert_term("id-1".to_string(), a_term(), "t".to_string())
            .await
            .expect("insert");

        assert!(db.delete_term("id-1".to_string()).await.expect("delete"));
        assert!(
            !db.delete_term("id-1".to_string())
                .await
                .expect("delete again")
        );
        assert!(db.list_terms().await.expect("list").is_empty());
    }

    #[tokio::test]
    async fn data_survives_reopening_the_same_file() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("flashcards.db");
        {
            let db = Db::open(&path).expect("open");
            db.insert_term("id-1".to_string(), a_term(), "t".to_string())
                .await
                .expect("insert");
        }
        let reopened = Db::open(&path).expect("reopen");
        assert_eq!(reopened.list_terms().await.expect("list").len(), 1);
    }
}
