//! `POST/DELETE/GET /api/session/learner`: current-learner cookie identity.
//! Learner-scoped operations derive identity from this cookie; the cookie
//! value is never sourced from a request body, so a caller cannot act as
//! another Learner by supplying an arbitrary ID (grilled-spec.md sec. 2,
//! sec. 5).

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum_extra::extract::CookieJar;
use chrono::Utc;
use serde::Deserialize;

use crate::envelope::{self, ErrorDetail, ErrorResponse, SuccessEnvelope};
use crate::identity;
use crate::learners::repository;
use crate::learners::{Learner, LearnerId};
use crate::routes::internal_error;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SelectLearnerRequest {
    pub learner_id: LearnerId,
}

/// Selects an existing Learner and sets the current-learner cookie
/// (spec.md story 2).
pub async fn select_learner(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<SelectLearnerRequest>,
) -> Result<(CookieJar, SuccessEnvelope<Learner>), ErrorResponse> {
    let learner = repository::find_by_id(state.db(), payload.learner_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| ErrorResponse {
            status_code: StatusCode::NOT_FOUND,
            envelope: envelope::error(
                "LEARNER_NOT_FOUND",
                "That learner profile no longer exists.",
                vec![ErrorDetail {
                    field: "learner_id".to_string(),
                    reason: "Not found".to_string(),
                }],
                Utc::now(),
            ),
        })?;

    let cookie = identity::learner_cookie(learner.id, state.config().cookie_lifetime_days);
    let jar = jar.add(cookie);

    Ok((jar, envelope::success(learner, Utc::now())))
}

/// Clears the current-learner cookie (spec.md story: sign-out / switch
/// profile).
pub async fn clear_learner_session(jar: CookieJar) -> (CookieJar, SuccessEnvelope<()>) {
    let jar = jar.remove(identity::learner_cookie_key());
    (jar, envelope::success_without_data(Utc::now()))
}

/// Resolves the current Learner from the cookie, if any. An invalid or
/// deleted-profile cookie is cleared here so the client can treat the
/// response uniformly and redirect to Home (spec.md story 4).
pub async fn get_current_learner(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, SuccessEnvelope<Option<Learner>>), ErrorResponse> {
    let cookie_value = jar
        .get(identity::LEARNER_COOKIE_NAME)
        .map(|cookie| cookie.value().to_string());

    let learner_id = cookie_value.as_deref().and_then(identity::parse_learner_id);

    let learner = match learner_id {
        Some(id) => repository::find_by_id(state.db(), id)
            .await
            .map_err(internal_error)?,
        None => None,
    };

    let jar = if cookie_value.is_some() && learner.is_none() {
        jar.remove(identity::learner_cookie_key())
    } else {
        jar
    };

    Ok((jar, envelope::success(learner, Utc::now())))
}
