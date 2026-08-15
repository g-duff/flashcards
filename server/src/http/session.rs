//! `POST/DELETE/GET /api/session/learner`: current-learner cookie identity.
//! Learner-scoped operations derive identity from this cookie; the cookie
//! value is never sourced from a request body, so a caller cannot act as
//! another Learner by supplying an arbitrary ID (grilled-spec.md sec. 2,
//! sec. 5).

use axum::Json;
use axum::extract::State;
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, StatusCode};
use chrono::Utc;
use serde::Deserialize;

use crate::http::cookies;
use crate::http::envelope::{self, ErrorDetail, ErrorResponse, SuccessEnvelope};
use crate::http::internal_error;
use crate::learners::repository;
use crate::learners::{Learner, LearnerId};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SelectLearnerRequest {
    pub learner_id: LearnerId,
}

/// Reads the raw `learner_id` cookie value out of the incoming request, if
/// that cookie is present at all. `Some(None)` means it was present but
/// held a non-numeric value; `None` means no `learner_id` cookie was sent.
fn learner_cookie_from_request(headers: &HeaderMap) -> Option<Option<LearnerId>> {
    let cookie_header = headers.get(COOKIE)?.to_str().ok()?;
    cookies::parse_learner_cookie_presence(cookie_header)
}

/// Selects an existing Learner and sets the current-learner cookie
/// (spec.md story 2).
pub async fn select_learner(
    State(state): State<AppState>,
    Json(payload): Json<SelectLearnerRequest>,
) -> Result<(HeaderMap, SuccessEnvelope<Learner>), ErrorResponse> {
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

    let mut headers = HeaderMap::new();
    headers.insert(
        SET_COOKIE,
        cookies::learner_cookie_header(learner.id, state.config().cookie_lifetime_days),
    );

    Ok((headers, envelope::success(learner, Utc::now())))
}

/// Clears the current-learner cookie (spec.md story: sign-out / switch
/// profile).
pub async fn clear_learner_session() -> (HeaderMap, SuccessEnvelope<()>) {
    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, cookies::clear_learner_cookie_header());

    (headers, envelope::success_without_data(Utc::now()))
}

/// Resolves the current Learner from the cookie, if any. An invalid or
/// deleted-profile cookie is cleared here so the client can treat the
/// response uniformly and redirect to Home (spec.md story 4).
pub async fn get_current_learner(
    State(state): State<AppState>,
    request_headers: HeaderMap,
) -> Result<(HeaderMap, SuccessEnvelope<Option<Learner>>), ErrorResponse> {
    let learner_cookie = learner_cookie_from_request(&request_headers);

    let learner = match learner_cookie.flatten() {
        Some(id) => repository::find_by_id(state.db(), id)
            .await
            .map_err(internal_error)?,
        None => None,
    };

    let mut response_headers = HeaderMap::new();
    if learner_cookie.is_some() && learner.is_none() {
        response_headers.insert(SET_COOKIE, cookies::clear_learner_cookie_header());
    }

    Ok((response_headers, envelope::success(learner, Utc::now())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learner_cookie_from_request_returns_none_when_no_cookie_header() {
        let headers = HeaderMap::new();

        let result = learner_cookie_from_request(&headers);

        assert_eq!(result, None);
    }

    #[test]
    fn learner_cookie_from_request_returns_none_when_learner_id_cookie_absent() {
        let mut headers = HeaderMap::new();
        headers.insert(COOKIE, "theme=dark; other=value".parse().unwrap());

        let result = learner_cookie_from_request(&headers);

        assert_eq!(result, None);
    }

    #[test]
    fn learner_cookie_from_request_returns_some_none_for_non_numeric_value() {
        let mut headers = HeaderMap::new();
        headers.insert(COOKIE, "learner_id=not-a-number".parse().unwrap());

        let result = learner_cookie_from_request(&headers);

        assert_eq!(result, Some(None));
    }

    #[test]
    fn learner_cookie_from_request_parses_valid_numeric_id() {
        let mut headers = HeaderMap::new();
        headers.insert(COOKIE, "theme=dark; learner_id=42; other=1".parse().unwrap());

        let result = learner_cookie_from_request(&headers);

        assert_eq!(result, Some(Some(LearnerId(42))));
    }
}
