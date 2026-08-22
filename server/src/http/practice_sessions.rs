//! `POST /api/practice-sessions`, `GET /api/practice-sessions/:id`:
//! Practice Session generation and reading (grilled-spec.md sec. 5;
//! ticket 07).

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use chrono::Utc;
use serde::Deserialize;

use crate::categories::CategoryId;
use crate::direction_progress;
use crate::http::cookies::learner_id_from_headers;
use crate::http::envelope::{self, ErrorDetail, ErrorResponse, SuccessEnvelope};
use crate::http::internal_error;
use crate::practice_sessions::repository::{self, NewSession};
use crate::practice_sessions::{
    self, GenerateSessionInput, PracticeSession, PracticeSessionId, QuestionCountError,
};
use crate::state::AppState;
use crate::translation_direction::TranslationDirection;
use crate::vocabulary_entries::repository::{self as vocabulary_entries_repository, ListFilter};

#[derive(Debug, Deserialize)]
pub struct CreatePracticeSessionRequest {
    pub category_id: CategoryId,
    pub direction: TranslationDirection,
    pub question_count: u32,
}

/// Creates an active Practice Session for the current Learner: generates
/// its Question set from Eligible Entries in the requested Category and
/// Direction, and immutably snapshots it (grilled-spec.md sec. 4, 5;
/// ticket 07).
pub async fn create_practice_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreatePracticeSessionRequest>,
) -> Result<(StatusCode, SuccessEnvelope<PracticeSession>), ErrorResponse> {
    let learner_id = learner_id_from_headers(&headers).ok_or_else(no_current_learner_error)?;

    let config = state.config();
    let question_count = practice_sessions::validate_question_count(
        payload.question_count,
        config.question_count_min,
        config.question_count_max,
    )
    .map_err(question_count_error)?;

    let category_entries = vocabulary_entries_repository::list(
        state.db(),
        ListFilter {
            category_id: Some(payload.category_id),
            source_language: None,
            target_language: None,
        },
    )
    .await
    .map_err(internal_error)?;

    let all_entries = vocabulary_entries_repository::list(
        state.db(),
        ListFilter {
            category_id: None,
            source_language: None,
            target_language: None,
        },
    )
    .await
    .map_err(internal_error)?;

    let last_correct_at = direction_progress::repository::last_correct_at_by_entry_and_direction(
        state.db(),
        learner_id,
    )
    .await
    .map_err(internal_error)?;

    let now = Utc::now();
    let questions = {
        let mut rng = rand::thread_rng();
        practice_sessions::generate_session_questions(
            GenerateSessionInput {
                category_id: payload.category_id,
                direction: payload.direction,
                requested_count: question_count.0,
                category_entries: &category_entries,
                all_entries: &all_entries,
                last_correct_at: &last_correct_at,
                min_interval_before_retest_days: config
                    .algorithm_defaults
                    .min_interval_before_retest_days,
                incorrect_distractor_count: config.incorrect_distractor_count as usize,
                now,
            },
            &mut rng,
        )
    };

    if questions.is_empty() {
        return Err(no_eligible_questions_error());
    }

    let session = repository::create(
        state.db(),
        NewSession {
            learner_id,
            category_id: payload.category_id,
            direction: payload.direction,
            requested_question_count: question_count.0,
            questions: &questions,
        },
        now,
    )
    .await
    .map_err(internal_error)?;

    tracing::info!(
        practice_session_id = %session.id,
        learner_id = %learner_id,
        question_count = session.actual_question_count,
        "practice session created"
    );

    Ok((StatusCode::CREATED, envelope::success(session, Utc::now())))
}

/// Reads a Practice Session's immutable snapshot and status
/// (grilled-spec.md sec. 5; ticket 07).
pub async fn get_practice_session(
    State(state): State<AppState>,
    Path(id): Path<PracticeSessionId>,
) -> Result<SuccessEnvelope<PracticeSession>, ErrorResponse> {
    let session = repository::find_by_id(state.db(), id)
        .await
        .map_err(internal_error)?
        .ok_or_else(not_found_error)?;

    Ok(envelope::success(session, Utc::now()))
}

fn no_current_learner_error() -> ErrorResponse {
    ErrorResponse {
        status_code: StatusCode::BAD_REQUEST,
        envelope: envelope::error(
            "LEARNER_NOT_SELECTED",
            "No current learner is selected.",
            vec![ErrorDetail {
                field: "learner_id".to_string(),
                reason: "No current-learner cookie was present".to_string(),
            }],
            Utc::now(),
        ),
    }
}

fn question_count_error(error: QuestionCountError) -> ErrorResponse {
    let QuestionCountError::OutOfRange { min, max } = error;
    ErrorResponse {
        status_code: StatusCode::BAD_REQUEST,
        envelope: envelope::error(
            "VALIDATION_ERROR",
            "Invalid practice session request.",
            vec![ErrorDetail {
                field: "question_count".to_string(),
                reason: format!("Must be between {min} and {max}"),
            }],
            Utc::now(),
        ),
    }
}

fn no_eligible_questions_error() -> ErrorResponse {
    ErrorResponse {
        status_code: StatusCode::CONFLICT,
        envelope: envelope::error(
            "NO_ELIGIBLE_QUESTIONS",
            "No eligible vocabulary is available for this category and direction right now.",
            vec![],
            Utc::now(),
        ),
    }
}

fn not_found_error() -> ErrorResponse {
    ErrorResponse {
        status_code: StatusCode::NOT_FOUND,
        envelope: envelope::error(
            "PRACTICE_SESSION_NOT_FOUND",
            "That practice session no longer exists.",
            vec![ErrorDetail {
                field: "id".to_string(),
                reason: "Not found".to_string(),
            }],
            Utc::now(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_current_learner_error_returns_400() {
        let error = no_current_learner_error();

        assert_eq!(error.status_code, StatusCode::BAD_REQUEST);
        assert_eq!(error.envelope.error.code, "LEARNER_NOT_SELECTED");
    }

    #[test]
    fn question_count_error_returns_400() {
        let error = question_count_error(QuestionCountError::OutOfRange { min: 10, max: 20 });

        assert_eq!(error.status_code, StatusCode::BAD_REQUEST);
        assert_eq!(error.envelope.error.code, "VALIDATION_ERROR");
    }

    #[test]
    fn no_eligible_questions_error_returns_409() {
        let error = no_eligible_questions_error();

        assert_eq!(error.status_code, StatusCode::CONFLICT);
        assert_eq!(error.envelope.error.code, "NO_ELIGIBLE_QUESTIONS");
    }

    #[test]
    fn not_found_error_returns_404() {
        let error = not_found_error();

        assert_eq!(error.status_code, StatusCode::NOT_FOUND);
        assert_eq!(error.envelope.error.code, "PRACTICE_SESSION_NOT_FOUND");
    }
}
