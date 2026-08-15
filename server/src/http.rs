//! HTTP layer: route wiring plus the pure helpers (cookie headers, response
//! envelope) that shape requests and responses. The imperative shell's HTTP
//! boundary, as distinct from domain logic (`learners`, `config`) and
//! infrastructure (`db`, `state`).

pub mod cookies;
pub mod envelope;
pub mod health;
pub mod learners;
pub mod session;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
use chrono::Utc;

use self::envelope::ErrorResponse;
use crate::learners::repository::RepositoryError;
use crate::state::AppState;

/// Maps an unexpected repository failure to the standard `500` envelope,
/// logging the underlying cause. Shared by every route module that talks to
/// the Learner repository.
pub(crate) fn internal_error(source: RepositoryError) -> ErrorResponse {
    tracing::error!(error = %source, "learner repository operation failed");
    ErrorResponse {
        status_code: StatusCode::INTERNAL_SERVER_ERROR,
        envelope: envelope::error(
            "INTERNAL_ERROR",
            "Something went wrong. Please try again.",
            vec![],
            Utc::now(),
        ),
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health::get_health))
        .route(
            "/api/learners",
            get(learners::list_learners).post(learners::create_learner),
        )
        .route(
            "/api/learners/{id}",
            axum::routing::patch(learners::rename_learner).delete(learners::delete_learner),
        )
        .route(
            "/api/session/learner",
            get(session::get_current_learner)
                .post(session::select_learner)
                .delete(session::clear_learner_session),
        )
        .with_state(state)
}
