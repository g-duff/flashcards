use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use chrono::Utc;

use crate::core;
use crate::http::AppState;
use crate::http::error::AppError;
use crate::model::{Deleted, NewTerm, NotesPatch, Term};

pub async fn healthz() -> &'static str {
    "ok"
}

/// The OpenAPI 3.0 spec for this API, compiled into the binary so it can
/// never drift from the build that serves it. Rendered by the shared
/// Swagger UI at /docs/ (see the Sandy Bank components/swagger-ui),
/// which loads it via the nginx-routed path /flashcards/openapi.yaml.
pub async fn openapi() -> impl axum::response::IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "application/yaml")],
        include_str!("../../openapi.yaml"),
    )
}

/// Every Term, oldest first. No pagination — a personal deck stays small.
pub async fn list_terms(State(state): State<AppState>) -> Result<Json<Vec<Term>>, AppError> {
    state.db.list_terms().await.map(Json).map_err(internal)
}

/// Add a Term. The id is derived from the three texts (see
/// [`core::term_id`]), so re-posting the same Term is idempotent — the
/// already-stored row comes back, no duplicate.
pub async fn create_term(
    State(state): State<AppState>,
    body: Result<Json<NewTerm>, JsonRejection>,
) -> Result<Json<Term>, AppError> {
    let Json(req) = body.map_err(|rej| AppError::BadRequest(rej.body_text()))?;
    core::validate_new_term(&req).map_err(|e| AppError::BadRequest(e.to_string()))?;

    let id = core::term_id(&req).to_string();
    let created_at = Utc::now().to_rfc3339();
    let term = state
        .db
        .insert_term(id, req, created_at)
        .await
        .map_err(internal)?;

    // "stored", not "created": an identical Term already present is
    // returned as-is (idempotent id), and no new row is written.
    tracing::info!(term_id = %term.id, "term stored");
    Ok(Json(term))
}

/// Edit a Term's `notes` — its only mutable field.
pub async fn patch_term(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Result<Json<NotesPatch>, JsonRejection>,
) -> Result<Json<Term>, AppError> {
    let Json(patch) = body.map_err(|rej| AppError::BadRequest(rej.body_text()))?;
    match state
        .db
        .patch_term_notes(id, patch.notes)
        .await
        .map_err(internal)?
    {
        Some(term) => {
            tracing::info!(term_id = %term.id, "term notes updated");
            Ok(Json(term))
        }
        None => Err(AppError::NotFound),
    }
}

/// Delete a Term.
pub async fn delete_term(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Deleted>, AppError> {
    if state.db.delete_term(id.clone()).await.map_err(internal)? {
        tracing::info!(term_id = %id, "term deleted");
        Ok(Json(Deleted { deleted: id }))
    } else {
        Err(AppError::NotFound)
    }
}

/// Map an infrastructure error to a 500, logging the cause at the
/// boundary (per `CODING_STANDARDS.md` § Logging).
fn internal<E: std::fmt::Display>(err: E) -> AppError {
    tracing::error!(error = %err, "store operation failed");
    AppError::Internal
}
