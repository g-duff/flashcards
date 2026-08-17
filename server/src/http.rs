//! HTTP layer: route wiring plus the pure helpers (cookie headers, response
//! envelope) that shape requests and responses. The imperative shell's HTTP
//! boundary, as distinct from domain logic (`learners`, `config`) and
//! infrastructure (`db`, `state`).

pub mod categories;
pub mod cookies;
pub mod envelope;
pub mod health;
pub mod learners;
pub mod session;
pub mod vocabulary_entries;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
use chrono::Utc;

use self::envelope::ErrorResponse;
use crate::state::AppState;

/// Maps an unexpected repository failure to the standard `500` envelope,
/// logging the underlying cause. Shared by every route module that talks to
/// a repository.
pub(crate) fn internal_error(source: impl std::error::Error) -> ErrorResponse {
    tracing::error!(error = %source, "repository operation failed");
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
            "/api/categories",
            get(categories::list_categories).post(categories::create_category),
        )
        .route(
            "/api/categories/{id}",
            get(categories::get_category)
                .patch(categories::rename_category)
                .delete(categories::delete_category),
        )
        .route(
            "/api/vocabulary-entries",
            get(vocabulary_entries::list_vocabulary_entries)
                .post(vocabulary_entries::create_vocabulary_entry),
        )
        .route(
            "/api/vocabulary-entries/bulk",
            axum::routing::post(vocabulary_entries::create_vocabulary_entries_bulk),
        )
        .route(
            "/api/vocabulary-entries/{id}",
            get(vocabulary_entries::get_vocabulary_entry)
                .patch(vocabulary_entries::update_vocabulary_entry)
                .delete(vocabulary_entries::delete_vocabulary_entry),
        )
        .route(
            "/api/session/learner",
            get(session::get_current_learner)
                .post(session::select_learner)
                .delete(session::clear_learner_session),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_error_returns_500_with_correct_error_code() {
        let error = internal_error(crate::learners::repository::RepositoryError::DuplicateName);

        assert_eq!(error.status_code, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(error.envelope.error.code, "INTERNAL_ERROR");
        assert_eq!(
            error.envelope.error.message,
            "Something went wrong. Please try again."
        );
        assert!(error.envelope.error.details.is_empty());
    }
}
