//! Learner domain: durable local profiles with unique display names
//! (grilled-spec.md sec. 2).

pub mod repository;

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

/// A Learner's durable identity. Newtype over the raw row ID so a caller
/// cannot accidentally pass an unrelated ID (e.g. a Category ID) where a
/// Learner ID is expected (server/CODING_STANDARDS.md sec. 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct LearnerId(pub i64);

impl fmt::Display for LearnerId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Learner {
    pub id: LearnerId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A Learner display name that has passed validation, paired with the
/// normalized form used for case/whitespace-insensitive uniqueness checks.
#[derive(Debug, Clone, PartialEq)]
pub struct LearnerName {
    pub display: String,
    pub normalized: String,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LearnerNameError {
    #[error("name must not be empty")]
    Empty,
}

/// Validates and normalizes a raw Learner display name. Trims surrounding
/// whitespace, rejects an empty result, and derives the normalized form
/// (Unicode NFC + lowercase) used for duplicate detection while preserving
/// the trimmed display casing (grilled-spec.md sec. 2).
pub fn validate_name(raw: &str) -> Result<LearnerName, LearnerNameError> {
    let display = raw.trim().to_string();
    if display.is_empty() {
        return Err(LearnerNameError::Empty);
    }
    let normalized = display.nfc().collect::<String>().to_lowercase();
    Ok(LearnerName {
        display,
        normalized,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Learner names are compared case-insensitively after trimming
    // whitespace, while display casing is preserved (spec.md story 6).
    #[test]
    fn trims_whitespace_and_preserves_display_case() {
        let name = validate_name("  Alice  ").unwrap();

        assert_eq!(name.display, "Alice");
        assert_eq!(name.normalized, "alice");
    }

    #[test]
    fn rejects_empty_name_after_trimming() {
        let result = validate_name("   ");

        assert_eq!(result, Err(LearnerNameError::Empty));
    }

    // Surrounding whitespace and Unicode form are normalized for identity
    // checks (grilled-spec.md sec. 2).
    #[test]
    fn normalizes_unicode_form_for_duplicate_checks() {
        let precomposed = validate_name("Jose\u{0301}").unwrap();
        let decomposed = validate_name("José").unwrap();

        assert_eq!(precomposed.normalized, decomposed.normalized);
    }

    #[test]
    fn normalizes_case_for_duplicate_checks() {
        let lower = validate_name("alice").unwrap();
        let upper = validate_name("ALICE").unwrap();

        assert_eq!(lower.normalized, upper.normalized);
    }
}
