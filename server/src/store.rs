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

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use rusqlite_migration::{HookResult, M, Migrations};

use crate::core::{self, CardSeed, Leitner, Schedule};
use crate::model::{NewTerm, PracticeCard, PromptSide, Term};

/// Embedded schema history. v1 creates the `term` table; v2 adds the
/// `card` and `review` tables and, via its up-hook, back-fills the two
/// Cards for every Term that predates them (ticket 01 rows).
static MIGRATIONS: LazyLock<Migrations<'static>> = LazyLock::new(|| {
    Migrations::new(vec![
        M::up(include_str!("../migrations/001_terms.sql")),
        M::up_with_hook(
            include_str!("../migrations/002_cards_reviews.sql"),
            |tx: &rusqlite::Transaction<'_>| -> HookResult {
                // A migration is frozen history and has no access to the
                // request-path `Scheduler` seam, so it names `Leitner`
                // directly — the only v1 strategy.
                backfill_cards(tx, Utc::now())?;
                Ok(())
            },
        ),
    ])
});

/// The `SELECT ... FROM ...` a [`PracticeCard`] is built from — every
/// column [`row_to_practice_card`] reads, in order. Shared by the
/// due-list query and the post-review fetch, which differ only in their
/// `WHERE` clause.
const PRACTICE_CARD_SELECT: &str = "SELECT \
     c.id, c.term_id, c.prompt_side, c.due_at, c.schedule_state, \
     t.foreign_text, t.pivot_text, t.notes \
     FROM card c JOIN term t ON t.id = c.term_id";

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

    /// Insert a Term under its precomputed `id` together with its two
    /// Cards, all in one transaction. Idempotent: re-inserting the same
    /// `id` (Term or Card) is a no-op — `ON CONFLICT DO NOTHING` — and the
    /// already-stored Term is returned unchanged.
    pub async fn insert_term(
        &self,
        id: String,
        new: NewTerm,
        created_at: String,
        cards: Vec<CardSeed>,
    ) -> rusqlite::Result<Term> {
        self.with(move |conn| {
            let tx = conn.transaction()?;
            tx.execute(
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
            for card in &cards {
                insert_card_row(&tx, &id, card, &created_at)?;
            }
            let term = fetch_term(&tx, &id)?;
            tx.commit()?;
            Ok(term)
        })
        .await
    }

    /// Practice Cards, oldest-due first. `due_before` (an ISO-8601 string)
    /// keeps only Cards due at or before it; `limit` caps the count. Both
    /// are optional — with neither, every Card comes back.
    pub async fn due_cards(
        &self,
        due_before: Option<String>,
        limit: Option<i64>,
    ) -> rusqlite::Result<Vec<PracticeCard>> {
        self.with(move |conn| {
            let sql = format!(
                "{PRACTICE_CARD_SELECT} \
                 WHERE (?1 IS NULL OR c.due_at <= ?1) \
                 ORDER BY c.due_at, c.id \
                 LIMIT ?2"
            );
            let mut stmt = conn.prepare(&sql)?;
            let cards = stmt
                .query_map(
                    params![due_before, limit.unwrap_or(-1)],
                    row_to_practice_card,
                )?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(cards)
        })
        .await
    }

    /// The current opaque `schedule_state` of one Card, or `None` if no
    /// Card has that `id`. The caller feeds it to `Scheduler::on_review`.
    pub async fn card_schedule_state(&self, id: String) -> rusqlite::Result<Option<String>> {
        self.with(move |conn| {
            conn.query_row(
                "SELECT schedule_state FROM card WHERE id = ?1",
                [&id],
                |row| row.get(0),
            )
            .optional()
        })
        .await
    }

    /// Record one review of a Card in a single transaction: append the
    /// `review` row, write the rescheduled state (`next.state` +
    /// `next.due_at`) back to the Card, and return the updated
    /// [`PracticeCard`]. `None` if the Card vanished before the write
    /// (unknown id → the handler maps it to `404`).
    pub async fn record_review(
        &self,
        review_id: String,
        card_id: String,
        rating: &'static str,
        reviewed_at: String,
        next: Schedule,
    ) -> rusqlite::Result<Option<PracticeCard>> {
        self.with(move |conn| {
            let tx = conn.transaction()?;
            let known: Option<i64> = tx
                .query_row("SELECT 1 FROM card WHERE id = ?1", [&card_id], |row| {
                    row.get(0)
                })
                .optional()?;
            if known.is_none() {
                return Ok(None);
            }
            tx.execute(
                "INSERT INTO review (id, card_id, rating, reviewed_at) VALUES (?1, ?2, ?3, ?4)",
                params![review_id, card_id, rating, reviewed_at],
            )?;
            tx.execute(
                "UPDATE card SET schedule_state = ?2, due_at = ?3 WHERE id = ?1",
                params![card_id, next.state, next.due_at.to_rfc3339()],
            )?;
            let card = fetch_practice_card(&tx, &card_id)?;
            tx.commit()?;
            Ok(Some(card))
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

/// Insert one Card row, skipping it if its (deterministic) id is already
/// present. Shared by [`Db::insert_term`] and [`backfill_cards`].
fn insert_card_row(
    conn: &Connection,
    term_id: &str,
    seed: &CardSeed,
    created_at: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO card (id, term_id, prompt_side, due_at, schedule_state, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(id) DO NOTHING",
        params![
            seed.id,
            term_id,
            seed.prompt_side.as_str(),
            seed.schedule.due_at.to_rfc3339(),
            seed.schedule.state,
            created_at,
        ],
    )?;
    Ok(())
}

/// Create the two Cards for every Term already in the database. Runs once,
/// from the v2 migration's up-hook: new Terms get their Cards in
/// [`Db::insert_term`], but rows inserted under ticket 01 predate the
/// `card` table. Idempotent via `insert_card_row`.
fn backfill_cards(conn: &Connection, now: DateTime<Utc>) -> rusqlite::Result<()> {
    let term_ids: Vec<String> = {
        let mut stmt = conn.prepare("SELECT id FROM term")?;
        stmt.query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    let created_at = now.to_rfc3339();
    for term_id in &term_ids {
        for seed in core::card_seeds(term_id, &Leitner, now) {
            insert_card_row(conn, term_id, &seed, &created_at)?;
        }
    }
    Ok(())
}

fn fetch_practice_card(conn: &Connection, card_id: &str) -> rusqlite::Result<PracticeCard> {
    let sql = format!("{PRACTICE_CARD_SELECT} WHERE c.id = ?1");
    conn.query_row(&sql, [card_id], row_to_practice_card)
}

/// Map a joined `card` + `term` row to a [`PracticeCard`]: resolve the
/// prompt/answer texts by side and read the display box out of the opaque
/// schedule state.
fn row_to_practice_card(row: &rusqlite::Row<'_>) -> rusqlite::Result<PracticeCard> {
    let prompt_side_raw: String = row.get(2)?;
    let prompt_side = prompt_side_raw.parse::<PromptSide>().map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, err.into())
    })?;
    let schedule_state: String = row.get(4)?;
    let foreign_text: String = row.get(5)?;
    let pivot_text: String = row.get(6)?;
    let (prompt, answer) = core::prompt_and_answer(prompt_side, &foreign_text, &pivot_text);
    Ok(PracticeCard {
        id: row.get(0)?,
        term_id: row.get(1)?,
        prompt_side,
        prompt,
        answer,
        notes: row.get(7)?,
        due_at: row.get(3)?,
        box_number: Leitner::box_of(&schedule_state).unwrap_or(1),
    })
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
    use crate::core::Scheduler;
    use crate::model::Rating;
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

    /// A fixed instant — the store's schedule inputs are all parameters,
    /// so tests pin their own clock.
    fn now() -> DateTime<Utc> {
        chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 1, 1, 12, 0, 0).unwrap()
    }

    /// The two Card seeds a Term of `id` yields, from the v1 scheduler.
    fn seeds(id: &str) -> Vec<CardSeed> {
        core::card_seeds(id, &Leitner, now()).to_vec()
    }

    /// Insert `a_term()` under `id` with its two Cards.
    async fn insert_a_term(db: &Db, id: &str) -> Term {
        db.insert_term(id.to_string(), a_term(), now().to_rfc3339(), seeds(id))
            .await
            .expect("insert")
    }

    #[tokio::test]
    async fn insert_then_list_roundtrips_the_term() {
        let (_dir, db) = temp_db();
        let stored = insert_a_term(&db, "id-1").await;

        assert_eq!(stored.foreign_text, "perro");
        assert_eq!(stored.notes.as_deref(), Some("el perro (m)"));
        assert_eq!(db.list_terms().await.expect("list"), vec![stored]);
    }

    #[tokio::test]
    async fn insert_term_creates_two_cards_one_per_side() {
        let (_dir, db) = temp_db();
        insert_a_term(&db, "id-1").await;

        let mut cards = db.due_cards(None, None).await.expect("cards");
        cards.sort_by(|a, b| a.prompt_side.as_str().cmp(b.prompt_side.as_str()));
        assert_eq!(cards.len(), 2);

        assert_eq!(cards[0].prompt_side, PromptSide::Foreign);
        assert_eq!(cards[0].prompt, "perro");
        assert_eq!(cards[0].answer, "dog");
        assert_eq!(cards[1].prompt_side, PromptSide::Pivot);
        assert_eq!(cards[1].prompt, "dog");
        assert_eq!(cards[1].answer, "perro");
        for card in &cards {
            assert_eq!(card.box_number, 1);
            assert_eq!(card.notes.as_deref(), Some("el perro (m)"));
            assert_eq!(card.term_id, "id-1");
        }
    }

    #[tokio::test]
    async fn re_inserting_the_same_id_is_a_no_op() {
        let (_dir, db) = temp_db();
        let first = insert_a_term(&db, "id-1").await;

        let second = db
            .insert_term(
                "id-1".to_string(),
                NewTerm {
                    notes: Some("a different note".to_string()),
                    ..a_term()
                },
                "2027-09-09T00:00:00Z".to_string(),
                seeds("id-1"),
            )
            .await
            .expect("second insert");

        assert_eq!(first, second, "the originally stored row wins");
        assert_eq!(db.list_terms().await.expect("list").len(), 1);
        assert_eq!(db.due_cards(None, None).await.expect("cards").len(), 2);
    }

    #[tokio::test]
    async fn patch_notes_changes_only_notes_and_can_clear_them() {
        let (_dir, db) = temp_db();
        insert_a_term(&db, "id-1").await;

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
        insert_a_term(&db, "id-1").await;

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
            insert_a_term(&db, "id-1").await;
        }
        let reopened = Db::open(&path).expect("reopen");
        assert_eq!(reopened.list_terms().await.expect("list").len(), 1);
        assert_eq!(
            reopened.due_cards(None, None).await.expect("cards").len(),
            2
        );
    }

    #[tokio::test]
    async fn due_cards_filters_by_due_before_orders_and_limits() {
        let (_dir, db) = temp_db();
        // Three Terms → six Cards, all initially due at `now()`.
        for id in ["a", "b", "c"] {
            insert_a_term(&db, id).await;
        }
        // Push one Card's due date far out by reviewing it with a pass.
        let all = db.due_cards(None, None).await.expect("all");
        let bumped = &all[0];
        let next = Leitner
            .on_review(r#"{"box":1}"#, Rating::Pass, now())
            .unwrap();
        db.record_review(
            "rev-1".to_string(),
            bumped.id.clone(),
            "pass",
            now().to_rfc3339(),
            next,
        )
        .await
        .expect("review")
        .expect("card");

        // A cut-off just after `now()` excludes the bumped Card.
        let cutoff = (now() + chrono::Duration::minutes(1)).to_rfc3339();
        let due = db.due_cards(Some(cutoff.clone()), None).await.expect("due");
        assert_eq!(due.len(), 5);
        assert!(due.iter().all(|c| c.id != bumped.id));
        // Ordered by due_at ascending.
        assert!(due.windows(2).all(|w| w[0].due_at <= w[1].due_at));
        // `limit` caps the list.
        let capped = db.due_cards(Some(cutoff), Some(2)).await.expect("capped");
        assert_eq!(capped.len(), 2);
    }

    #[tokio::test]
    async fn record_review_pass_moves_due_out_and_promotes_the_box() {
        let (_dir, db) = temp_db();
        insert_a_term(&db, "id-1").await;
        let card = db.due_cards(None, None).await.expect("cards")[0].clone();

        let next = Leitner
            .on_review(r#"{"box":1}"#, Rating::Pass, now())
            .unwrap();
        let updated = db
            .record_review(
                "rev-1".to_string(),
                card.id.clone(),
                "pass",
                now().to_rfc3339(),
                next,
            )
            .await
            .expect("review")
            .expect("card");

        assert_eq!(updated.box_number, 2);
        assert!(updated.due_at > card.due_at, "due date moved out");
    }

    #[tokio::test]
    async fn record_review_fail_resets_the_box() {
        let (_dir, db) = temp_db();
        insert_a_term(&db, "id-1").await;
        let card_id = db.due_cards(None, None).await.expect("cards")[0].id.clone();

        // Promote to box 3 first.
        let mut state = r#"{"box":1}"#.to_string();
        for i in 0..2 {
            let next = Leitner.on_review(&state, Rating::Pass, now()).unwrap();
            state = next.state.clone();
            db.record_review(
                format!("rev-pass-{i}"),
                card_id.clone(),
                "pass",
                now().to_rfc3339(),
                next,
            )
            .await
            .expect("review")
            .expect("card");
        }

        let reset = Leitner.on_review(&state, Rating::Fail, now()).unwrap();
        let updated = db
            .record_review(
                "rev-fail".to_string(),
                card_id,
                "fail",
                now().to_rfc3339(),
                reset,
            )
            .await
            .expect("review")
            .expect("card");

        assert_eq!(updated.box_number, 1);
    }

    #[tokio::test]
    async fn record_review_on_an_unknown_card_is_none() {
        let (_dir, db) = temp_db();
        let next = Schedule {
            state: r#"{"box":2}"#.to_string(),
            due_at: now(),
        };
        let outcome = db
            .record_review(
                "rev-1".to_string(),
                "no-such-card".to_string(),
                "pass",
                now().to_rfc3339(),
                next,
            )
            .await
            .expect("review");
        assert!(outcome.is_none());
    }

    #[tokio::test]
    async fn deleting_a_term_cascades_to_its_cards_and_reviews() {
        let (_dir, db) = temp_db();
        insert_a_term(&db, "id-1").await;
        let card_id = db.due_cards(None, None).await.expect("cards")[0].id.clone();
        let next = Schedule {
            state: r#"{"box":2}"#.to_string(),
            due_at: now(),
        };
        db.record_review(
            "rev-1".to_string(),
            card_id.clone(),
            "pass",
            now().to_rfc3339(),
            next,
        )
        .await
        .expect("review")
        .expect("card");

        assert!(db.delete_term("id-1".to_string()).await.expect("delete"));

        assert!(db.due_cards(None, None).await.expect("cards").is_empty());
        let review_count: i64 = db
            .with(|conn| conn.query_row("SELECT count(*) FROM review", [], |r| r.get(0)))
            .await
            .expect("count");
        assert_eq!(review_count, 0, "reviews went with the card");
    }

    #[tokio::test]
    async fn backfill_cards_creates_the_pair_for_a_pre_existing_term() {
        let (_dir, db) = temp_db();
        // Simulate a ticket-01 Term row with no Cards.
        db.with(|conn| {
            conn.execute(
                "INSERT INTO term (id, foreign_lang, foreign_text, pivot_text, notes, created_at) \
                 VALUES ('old-1', 'es', 'gato', 'cat', NULL, 't')",
                [],
            )?;
            conn.execute("DELETE FROM card WHERE term_id = 'old-1'", [])
        })
        .await
        .expect("seed row");

        db.with(|conn| {
            let tx = conn.transaction()?;
            backfill_cards(&tx, now())?;
            tx.commit()
        })
        .await
        .expect("backfill");

        let cards = db.due_cards(None, None).await.expect("cards");
        assert_eq!(cards.len(), 2);
        assert!(cards.iter().any(|c| c.prompt == "gato"));
        assert!(cards.iter().any(|c| c.prompt == "cat"));

        // Running it again is a no-op (deterministic ids).
        db.with(|conn| {
            let tx = conn.transaction()?;
            backfill_cards(&tx, now())?;
            tx.commit()
        })
        .await
        .expect("backfill again");
        assert_eq!(db.due_cards(None, None).await.expect("cards").len(), 2);
    }
}
