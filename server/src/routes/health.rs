//! `GET /api/health`: reports whether the server is up and can reach its
//! database.

use axum::extract::State;
use axum::http::StatusCode;
use chrono::Utc;
use serde::Serialize;

use crate::envelope::{self, ErrorResponse, SuccessEnvelope};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct HealthData {
    pub status: &'static str,
}

pub async fn get_health(
    State(state): State<AppState>,
) -> Result<SuccessEnvelope<HealthData>, ErrorResponse> {
    sqlx::query("SELECT 1")
        .execute(state.db())
        .await
        .map_err(|source| {
            tracing::error!(error = %source, "health check database probe failed");
            ErrorResponse {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                envelope: envelope::error(
                    "DATABASE_UNAVAILABLE",
                    "The database is currently unreachable.",
                    vec![],
                    Utc::now(),
                ),
            }
        })?;

    Ok(envelope::success(HealthData { status: "ok" }, Utc::now()))
}
