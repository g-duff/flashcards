use axum::Json;
use axum::extract::{Path, State};

use crate::core;
use crate::http::AppState;
use crate::http::error::AppError;
use crate::model::{Card, NewCard};

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

/// Every card, insertion order. No pagination — a personal deck stays
/// small.
pub async fn list_cards(State(state): State<AppState>) -> Json<Vec<Card>> {
    Json(state.store.list())
}

pub async fn get_card(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<Card>, AppError> {
    state.store.get(id).map(Json).ok_or(AppError::NotFound)
}

pub async fn create_card(
    State(state): State<AppState>,
    Json(req): Json<NewCard>,
) -> Result<Json<Card>, AppError> {
    // Pure validation lives in the functional core; the handler just
    // maps a rejection onto a 400.
    core::validate_new_card(&req.front, &req.back)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let card = state.store.add(req.front, req.back);
    tracing::info!(card_id = card.id, "card created");
    Ok(Json(card))
}
