use axum::Json;
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::core;
use crate::http::AppState;
use crate::http::error::AppError;
use crate::model::{Deleted, NewReview, NewTerm, NotesPatch, PracticeCard, Term};

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
    let now = Utc::now();
    let cards = core::card_seeds(&id, state.scheduler.as_ref(), now).to_vec();
    let term = state
        .db
        .insert_term(id, req, now.to_rfc3339(), cards)
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
        None => Err(AppError::NotFound("term not found")),
    }
}

/// The `GET /cards` query. Both fields optional: with neither, every Card
/// is returned; `due_before` (ISO-8601) keeps only Cards due at or before
/// it and `limit` caps the count — that pair is how the practice screen
/// pulls its queue.
#[derive(Debug, Deserialize)]
pub struct DueQuery {
    pub due_before: Option<String>,
    pub limit: Option<i64>,
}

/// Practice Cards, oldest-due first — every Card, or the due queue when
/// `?due_before=&limit=` is given.
pub async fn list_cards(
    State(state): State<AppState>,
    query: Result<Query<DueQuery>, QueryRejection>,
) -> Result<Json<Vec<PracticeCard>>, AppError> {
    let Query(query) = query.map_err(|rej| AppError::BadRequest(rej.body_text()))?;

    // Normalise the bound to a UTC ISO-8601 string so the store's text
    // comparison against stored `due_at` (also UTC) is by instant, not by
    // the offset notation the client happened to send.
    let due_before = query
        .due_before
        .as_deref()
        .map(|raw| {
            DateTime::parse_from_rfc3339(raw)
                .map(|dt| dt.with_timezone(&Utc).to_rfc3339())
                .map_err(|err| {
                    AppError::BadRequest(format!("due_before is not an ISO-8601 timestamp: {err}"))
                })
        })
        .transpose()?;

    state
        .db
        .due_cards(due_before, query.limit)
        .await
        .map(Json)
        .map_err(internal)
}

/// Grade one attempt at a Card. In a single transaction the server
/// appends a `review` row, runs the `Scheduler` to reschedule the Card,
/// and returns the updated [`PracticeCard`]. Unknown id → `404`; a
/// `rating` outside `pass`/`fail` is rejected by the extractor → `400`.
pub async fn create_review(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Result<Json<NewReview>, JsonRejection>,
) -> Result<Json<PracticeCard>, AppError> {
    let Json(review) = body.map_err(|rej| AppError::BadRequest(rej.body_text()))?;
    let card_missing = || AppError::NotFound("card not found");

    let current = state
        .db
        .card_schedule_state(id.clone())
        .await
        .map_err(internal)?
        .ok_or_else(card_missing)?;

    let now = Utc::now();
    let next = state
        .scheduler
        .on_review(&current, review.rating, now)
        .map_err(internal)?;

    let updated = state
        .db
        .record_review(
            Uuid::now_v7().to_string(),
            id.clone(),
            review.rating.as_str(),
            now.to_rfc3339(),
            next,
        )
        .await
        .map_err(internal)?
        .ok_or_else(card_missing)?;

    tracing::info!(card_id = %id, rating = review.rating.as_str(), "review recorded");
    Ok(Json(updated))
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
        Err(AppError::NotFound("term not found"))
    }
}

/// Map an infrastructure error to a 500, logging the cause at the
/// boundary (per `CODING_STANDARDS.md` § Logging).
fn internal<E: std::fmt::Display>(err: E) -> AppError {
    tracing::error!(error = %err, "store operation failed");
    AppError::Internal
}
