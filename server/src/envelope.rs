//! The success/error JSON envelope shared by every API response.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Meta {
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SuccessEnvelope<T> {
    pub status: SuccessStatus,
    pub data: Option<T>,
    pub meta: Meta,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SuccessStatus {
    Success,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ErrorDetail {
    pub field: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    pub details: Vec<ErrorDetail>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ErrorEnvelope {
    pub status: ErrorStatus,
    pub error: ErrorBody,
    pub meta: Meta,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorStatus {
    Error,
}

/// Builds a success envelope carrying `data`, timestamped `now`.
pub fn success<T>(data: T, now: DateTime<Utc>) -> SuccessEnvelope<T> {
    SuccessEnvelope {
        status: SuccessStatus::Success,
        data: Some(data),
        meta: Meta { timestamp: now },
    }
}

/// Builds a success envelope with no `data` payload, e.g. for a successful
/// deletion, timestamped `now`. Serializes as `data: null`.
pub fn success_without_data(now: DateTime<Utc>) -> SuccessEnvelope<()> {
    SuccessEnvelope {
        status: SuccessStatus::Success,
        data: None,
        meta: Meta { timestamp: now },
    }
}

/// Builds an error envelope, timestamped `now`. `details` may be empty, in
/// which case it serializes as `details: []`.
pub fn error(
    code: impl Into<String>,
    message: impl Into<String>,
    details: Vec<ErrorDetail>,
    now: DateTime<Utc>,
) -> ErrorEnvelope {
    ErrorEnvelope {
        status: ErrorStatus::Error,
        error: ErrorBody {
            code: code.into(),
            message: message.into(),
            details,
        },
        meta: Meta { timestamp: now },
    }
}

impl<T: Serialize> IntoResponse for SuccessEnvelope<T> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

/// An [`ErrorEnvelope`] paired with the HTTP status it should be returned
/// with, so handlers can construct both together.
pub struct ErrorResponse {
    pub status_code: StatusCode,
    pub envelope: ErrorEnvelope,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        (self.status_code, Json(self.envelope)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-08-11T17:56:16Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    // Success responses use a stable status/data/meta.timestamp shape
    // (grilled-spec.md sec. 5).
    #[test]
    fn success_envelope_serializes_with_data() {
        let envelope = success(serde_json::json!({"ok": true}), fixed_now());
        let value = serde_json::to_value(&envelope).unwrap();

        assert_eq!(value["status"], "success");
        assert_eq!(value["data"]["ok"], true);
        assert_eq!(value["meta"]["timestamp"], "2026-08-11T17:56:16Z");
    }

    // Successful deletion returns the success envelope with a `data: null`
    // field (grilled-spec.md sec. 5).
    #[test]
    fn success_without_data_includes_null_data_field() {
        let envelope = success_without_data(fixed_now());
        let value = serde_json::to_value(&envelope).unwrap();

        assert_eq!(value.get("data"), Some(&serde_json::Value::Null));
    }

    // Optional empty fields are always present and serialized as null/[]
    // rather than omitted (spec.md story 72).
    #[test]
    fn error_envelope_includes_empty_details_array() {
        let envelope = error("VALIDATION_ERROR", "Invalid input", vec![], fixed_now());
        let value = serde_json::to_value(&envelope).unwrap();

        assert_eq!(
            value["error"]["details"],
            serde_json::Value::Array(vec![])
        );
    }

    #[test]
    fn error_envelope_includes_populated_details() {
        let envelope = error(
            "VALIDATION_ERROR",
            "Invalid input",
            vec![ErrorDetail {
                field: "source_text".to_string(),
                reason: "Cannot be empty".to_string(),
            }],
            fixed_now(),
        );
        let value = serde_json::to_value(&envelope).unwrap();

        assert_eq!(value["error"]["details"][0]["field"], "source_text");
        assert_eq!(value["error"]["details"][0]["reason"], "Cannot be empty");
    }
}
