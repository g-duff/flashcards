//! `POST /api/learners`, `GET /api/learners`, `PATCH /api/learners/:id`,
//! `DELETE /api/learners/:id`: Learner creation, listing, renaming, and
//! deletion (grilled-spec.md sec. 5).

use axum::Json;
use axum::extract::{Path, State};
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use chrono::Utc;
use serde::Deserialize;

use crate::http::cookies;
use crate::http::envelope::{self, ErrorDetail, ErrorResponse, SuccessEnvelope};
use crate::http::internal_error;
use crate::learners::repository::{self, RepositoryError};
use crate::learners::{self, Learner, LearnerId};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateLearnerRequest {
    pub name: String,
}

/// Creates a Learner with a unique display name and sets the current-learner
/// cookie so the new profile becomes current immediately (spec.md story 1).
pub async fn create_learner(
    State(state): State<AppState>,
    Json(payload): Json<CreateLearnerRequest>,
) -> Result<
    (
        StatusCode,
        [(HeaderName, HeaderValue); 1],
        SuccessEnvelope<Learner>,
    ),
    ErrorResponse,
> {
    let name = learners::validate_name(&payload.name).map_err(|_| validation_error())?;

    let learner = repository::insert(state.db(), &name, Utc::now())
        .await
        .map_err(|error| match error {
            RepositoryError::DuplicateName => conflict_error(),
            other => internal_error(other),
        })?;

    let cookie = cookies::learner_cookie_header(learner.id, state.config().cookie_lifetime_days);

    Ok((
        StatusCode::CREATED,
        [(SET_COOKIE, cookie)],
        envelope::success(learner, Utc::now()),
    ))
}

/// Lists every Learner for profile selection on the Home screen
/// (spec.md story 2).
pub async fn list_learners(
    State(state): State<AppState>,
) -> Result<SuccessEnvelope<Vec<Learner>>, ErrorResponse> {
    let learners = repository::list(state.db()).await.map_err(internal_error)?;

    Ok(envelope::success(learners, Utc::now()))
}

#[derive(Debug, Deserialize)]
pub struct RenameLearnerRequest {
    pub name: String,
}

fn validation_error() -> ErrorResponse {
    ErrorResponse {
        status_code: StatusCode::BAD_REQUEST,
        envelope: envelope::error(
            "VALIDATION_ERROR",
            "Learner name cannot be empty.",
            vec![ErrorDetail {
                field: "name".to_string(),
                reason: "Cannot be empty".to_string(),
            }],
            Utc::now(),
        ),
    }
}

fn not_found_error() -> ErrorResponse {
    ErrorResponse {
        status_code: StatusCode::NOT_FOUND,
        envelope: envelope::error(
            "LEARNER_NOT_FOUND",
            "That learner profile no longer exists.",
            vec![ErrorDetail {
                field: "id".to_string(),
                reason: "Not found".to_string(),
            }],
            Utc::now(),
        ),
    }
}

fn conflict_error() -> ErrorResponse {
    ErrorResponse {
        status_code: StatusCode::CONFLICT,
        envelope: envelope::error(
            "LEARNER_NAME_CONFLICT",
            "A learner with this name already exists.",
            vec![ErrorDetail {
                field: "name".to_string(),
                reason: "Already in use".to_string(),
            }],
            Utc::now(),
        ),
    }
}

/// Renames a Learner, preserving its durable ID and all existing progress.
/// The new name is subject to the same case-insensitive, trimmed uniqueness
/// check as creation (spec.md story 5, 6; ticket 03).
pub async fn rename_learner(
    State(state): State<AppState>,
    Path(id): Path<LearnerId>,
    Json(payload): Json<RenameLearnerRequest>,
) -> Result<SuccessEnvelope<Learner>, ErrorResponse> {
    let name = learners::validate_name(&payload.name).map_err(|_| validation_error())?;

    let learner = repository::rename(state.db(), id, &name, Utc::now())
        .await
        .map_err(|error| match error {
            RepositoryError::DuplicateName => conflict_error(),
            other => internal_error(other),
        })?
        .ok_or_else(not_found_error)?;

    Ok(envelope::success(learner, Utc::now()))
}

/// Deletes a Learner and all personal dependent data, never touching shared
/// Categories or Vocabulary Entries. Clears the current-learner cookie when
/// the deleted Learner was current (spec.md story 7; ticket 03).
pub async fn delete_learner(
    State(state): State<AppState>,
    Path(id): Path<LearnerId>,
    request_headers: HeaderMap,
) -> Result<(HeaderMap, SuccessEnvelope<()>), ErrorResponse> {
    let deleted = repository::delete(state.db(), id)
        .await
        .map_err(internal_error)?;

    if !deleted {
        return Err(not_found_error());
    }

    let mut response_headers = HeaderMap::new();
    let current_cookie = request_headers
        .get(COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(cookies::parse_learner_cookie);
    if current_cookie == Some(id) {
        response_headers.insert(SET_COOKIE, cookies::clear_learner_cookie_header());
    }

    Ok((response_headers, envelope::success_without_data(Utc::now())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_error_returns_400_with_correct_error_code() {
        let error = validation_error();

        assert_eq!(error.status_code, StatusCode::BAD_REQUEST);
        assert_eq!(error.envelope.error.code, "VALIDATION_ERROR");
        assert_eq!(
            error.envelope.error.message,
            "Learner name cannot be empty."
        );
        assert_eq!(error.envelope.error.details.len(), 1);
        assert_eq!(error.envelope.error.details[0].field, "name");
        assert_eq!(error.envelope.error.details[0].reason, "Cannot be empty");
    }

    #[test]
    fn not_found_error_returns_404_with_correct_error_code() {
        let error = not_found_error();

        assert_eq!(error.status_code, StatusCode::NOT_FOUND);
        assert_eq!(error.envelope.error.code, "LEARNER_NOT_FOUND");
        assert_eq!(
            error.envelope.error.message,
            "That learner profile no longer exists."
        );
        assert_eq!(error.envelope.error.details.len(), 1);
        assert_eq!(error.envelope.error.details[0].field, "id");
        assert_eq!(error.envelope.error.details[0].reason, "Not found");
    }

    #[test]
    fn conflict_error_returns_409_with_correct_error_code() {
        let error = conflict_error();

        assert_eq!(error.status_code, StatusCode::CONFLICT);
        assert_eq!(error.envelope.error.code, "LEARNER_NAME_CONFLICT");
        assert_eq!(
            error.envelope.error.message,
            "A learner with this name already exists."
        );
        assert_eq!(error.envelope.error.details.len(), 1);
        assert_eq!(error.envelope.error.details[0].field, "name");
        assert_eq!(error.envelope.error.details[0].reason, "Already in use");
    }
}
