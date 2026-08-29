//! SQL persistence for Practice Sessions and their generated Questions.
//! Pure imperative shell: all I/O, no business logic.

use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use thiserror::Error;

use super::{
    GeneratedQuestion, OptionSnapshot, PracticeQuestion, PracticeQuestionId, PracticeSession,
    PracticeSessionId, SessionStatus,
};
use crate::categories::CategoryId;
use crate::learners::LearnerId;
use crate::translation_direction::TranslationDirection;
use crate::vocabulary_entries::VocabularyEntryId;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("stored practice session data was not well-formed: {0}")]
    Corrupt(String),
}

pub struct NewSession<'a> {
    pub learner_id: LearnerId,
    pub category_id: CategoryId,
    pub direction: TranslationDirection,
    pub requested_question_count: u32,
    pub questions: &'a [GeneratedQuestion],
}

/// Inserts a new active Practice Session and its immutable Question
/// snapshots in one transaction (grilled-spec.md sec. 2; ticket 07).
pub async fn create(
    pool: &SqlitePool,
    new: NewSession<'_>,
    now: DateTime<Utc>,
) -> Result<PracticeSession, RepositoryError> {
    let mut tx = pool.begin().await?;

    let session_row = sqlx::query_as::<_, SessionRow>(
        "INSERT INTO practice_sessions
            (learner_id, category_id, direction, status, requested_question_count,
             actual_question_count, answered_question_count, correct_count,
             started_at, completed_at, last_activity_at, created_at, updated_at)
         VALUES (?, ?, ?, 'active', ?, ?, 0, 0, ?, NULL, ?, ?, ?)
         RETURNING id, learner_id, category_id, direction, status, requested_question_count,
                   actual_question_count, answered_question_count, correct_count,
                   started_at, completed_at, last_activity_at, created_at, updated_at",
    )
    .bind(new.learner_id)
    .bind(new.category_id)
    .bind(new.direction.to_string())
    .bind(new.requested_question_count as i64)
    .bind(new.questions.len() as i64)
    .bind(now)
    .bind(now)
    .bind(now)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;

    let mut questions = Vec::with_capacity(new.questions.len());
    for (index, question) in new.questions.iter().enumerate() {
        let ordinal = (index + 1) as i64;
        let options_json = serde_json::to_string(&question.options)?;

        let question_row = sqlx::query_as::<_, QuestionRow>(
            "INSERT INTO practice_questions
                (session_id, vocabulary_entry_id, direction, ordinal,
                 prompt_text_snapshot, correct_text_snapshot, options_snapshot, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING id, session_id, vocabulary_entry_id, direction, ordinal,
                       prompt_text_snapshot, correct_text_snapshot, options_snapshot, created_at",
        )
        .bind(session_row.id)
        .bind(question.vocabulary_entry_id)
        .bind(question.direction.to_string())
        .bind(ordinal)
        .bind(&question.prompt_text)
        .bind(question.correct_text())
        .bind(&options_json)
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;

        questions.push(question_row.into_public()?);
    }

    tx.commit().await?;

    session_row.into_session(questions)
}

/// Reads a Practice Session and its Question snapshots by durable ID, if
/// one exists (ticket 07).
pub async fn find_by_id(
    pool: &SqlitePool,
    id: PracticeSessionId,
) -> Result<Option<PracticeSession>, RepositoryError> {
    let Some(session_row) = sqlx::query_as::<_, SessionRow>(
        "SELECT id, learner_id, category_id, direction, status, requested_question_count,
                actual_question_count, answered_question_count, correct_count,
                started_at, completed_at, last_activity_at, created_at, updated_at
         FROM practice_sessions WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };

    let question_rows = sqlx::query_as::<_, QuestionRow>(
        "SELECT id, session_id, vocabulary_entry_id, direction, ordinal,
                prompt_text_snapshot, correct_text_snapshot, options_snapshot, created_at
         FROM practice_questions WHERE session_id = ? ORDER BY ordinal ASC",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    let mut questions = Vec::with_capacity(question_rows.len());
    for row in question_rows {
        questions.push(row.into_public()?);
    }

    Ok(Some(session_row.into_session(questions)?))
}

#[derive(Debug, sqlx::FromRow)]
struct SessionRow {
    id: PracticeSessionId,
    learner_id: LearnerId,
    category_id: CategoryId,
    direction: String,
    status: String,
    requested_question_count: i64,
    actual_question_count: i64,
    answered_question_count: i64,
    correct_count: i64,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    last_activity_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SessionRow {
    fn into_session(
        self,
        questions: Vec<PracticeQuestion>,
    ) -> Result<PracticeSession, RepositoryError> {
        let direction = TranslationDirection::from_str(&self.direction)
            .map_err(|error| RepositoryError::Corrupt(error.to_string()))?;
        let status = SessionStatus::from_str(&self.status)
            .map_err(|error| RepositoryError::Corrupt(error.to_string()))?;

        Ok(PracticeSession {
            id: self.id,
            learner_id: self.learner_id,
            category_id: self.category_id,
            direction,
            status,
            requested_question_count: self.requested_question_count as u32,
            actual_question_count: self.actual_question_count as u32,
            answered_question_count: self.answered_question_count as u32,
            correct_count: self.correct_count as u32,
            started_at: self.started_at,
            completed_at: self.completed_at,
            last_activity_at: self.last_activity_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
            questions,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct QuestionRow {
    id: PracticeQuestionId,
    #[sqlx(rename = "session_id")]
    _session_id: PracticeSessionId,
    vocabulary_entry_id: VocabularyEntryId,
    direction: String,
    ordinal: i64,
    prompt_text_snapshot: String,
    #[sqlx(rename = "correct_text_snapshot")]
    _correct_text_snapshot: String,
    options_snapshot: String,
    #[sqlx(rename = "created_at")]
    _created_at: DateTime<Utc>,
}

impl QuestionRow {
    fn into_public(self) -> Result<PracticeQuestion, RepositoryError> {
        let direction = TranslationDirection::from_str(&self.direction)
            .map_err(|error| RepositoryError::Corrupt(error.to_string()))?;
        let options: Vec<OptionSnapshot> = serde_json::from_str(&self.options_snapshot)?;

        Ok(PracticeQuestion {
            id: self.id,
            vocabulary_entry_id: self.vocabulary_entry_id,
            direction,
            ordinal: self.ordinal as u32,
            prompt_text: self.prompt_text_snapshot,
            options: options.iter().map(OptionSnapshot::to_public).collect(),
        })
    }
}
