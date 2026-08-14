//! `POST /api/learners`, `GET /api/learners`: Learner creation and listing
//! for profile selection (grilled-spec.md sec. 5).

use axum::Json;
use axum::extract::State;
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use chrono::Utc;
use serde::Deserialize;

use crate::http::cookies;
use crate::http::envelope::{self, ErrorDetail, ErrorResponse, SuccessEnvelope};
use crate::http::internal_error;
use crate::learners::repository::{self, RepositoryError};
use crate::learners::{self, Learner};
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
    let name = learners::validate_name(&payload.name).map_err(|_| ErrorResponse {
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
    })?;

    let learner = repository::insert(state.db(), &name, Utc::now())
        .await
        .map_err(|error| match error {
            RepositoryError::DuplicateName => ErrorResponse {
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
            },
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
