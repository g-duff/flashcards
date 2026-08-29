//! SQL persistence for Direction-specific Progress. Pure imperative shell:
//! all I/O, no business logic.

use std::collections::HashMap;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use thiserror::Error;

use crate::learners::LearnerId;
use crate::translation_direction::TranslationDirection;
use crate::vocabulary_entries::VocabularyEntryId;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Loads the `last_correct_at` timestamp for every Vocabulary
/// Entry/Direction this Learner has answered correctly at least once,
/// keyed for the hard retest cooldown check in session generation (ticket
/// 07; grilled-spec.md sec. 4). Entries never answered correctly are simply
/// absent from the map.
pub async fn last_correct_at_by_entry_and_direction(
    pool: &SqlitePool,
    learner_id: LearnerId,
) -> Result<HashMap<(VocabularyEntryId, TranslationDirection), DateTime<Utc>>, RepositoryError> {
    let rows = sqlx::query_as::<_, LastCorrectRow>(
        "SELECT vocabulary_entry_id, direction, last_correct_at
         FROM direction_progress
         WHERE learner_id = ? AND last_correct_at IS NOT NULL",
    )
    .bind(learner_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let direction = TranslationDirection::from_str(&row.direction).ok()?;
            let last_correct_at = row.last_correct_at?;
            Some(((row.vocabulary_entry_id, direction), last_correct_at))
        })
        .collect())
}

#[derive(Debug, sqlx::FromRow)]
struct LastCorrectRow {
    vocabulary_entry_id: VocabularyEntryId,
    direction: String,
    last_correct_at: Option<DateTime<Utc>>,
}
