//! One error type for the HTTP layer, with its `IntoResponse`. Every
//! error body is `{ "error": "<message>" }`, matching the fleet
//! convention.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("term not found")]
    NotFound,
    /// An infrastructure failure (the database, mainly). The cause is
    /// logged where it is mapped; the client only ever sees this message.
    #[error("internal error")]
    Internal,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!({ "error": self.to_string() }))).into_response()
    }
}
