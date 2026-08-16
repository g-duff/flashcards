//! Category domain: shared organizational content available to every
//! Learner (grilled-spec.md sec. 2).

pub mod repository;

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

/// A Category's durable identity. Newtype over the raw row ID so a caller
/// cannot accidentally pass an unrelated ID (e.g. a Learner ID) where a
/// Category ID is expected (server/CODING_STANDARDS.md sec. 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct CategoryId(pub i64);

impl fmt::Display for CategoryId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Category {
    pub id: CategoryId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A Category name that has passed validation, paired with the normalized
/// form used for case/whitespace-insensitive uniqueness checks.
#[derive(Debug, Clone, PartialEq)]
pub struct CategoryName {
    pub display: String,
    pub normalized: String,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CategoryNameError {
    #[error("name must not be empty")]
    Empty,
}

/// Validates and normalizes a raw Category name. Trims surrounding
/// whitespace, rejects an empty result, and derives the normalized form
/// (Unicode NFC + lowercase) used for duplicate detection while preserving
/// the trimmed display casing (grilled-spec.md sec. 2).
pub fn validate_name(raw: &str) -> Result<CategoryName, CategoryNameError> {
    let display = raw.trim().to_string();
    if display.is_empty() {
        return Err(CategoryNameError::Empty);
    }
    let normalized = display.nfc().collect::<String>().to_lowercase();
    Ok(CategoryName {
        display,
        normalized,
    })
}

/// Ordering for `GET /api/categories` (ticket 04).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CategorySort {
    CreatedAt,
    Alphabetical,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Category names are compared case-insensitively after trimming
    // whitespace, while display casing is preserved (spec.md story 10).
    #[test]
    fn trims_whitespace_and_preserves_display_case() {
        let name = validate_name("  Animals  ").unwrap();

        assert_eq!(name.display, "Animals");
        assert_eq!(name.normalized, "animals");
    }

    #[test]
    fn rejects_empty_name_after_trimming() {
        let result = validate_name("   ");

        assert_eq!(result, Err(CategoryNameError::Empty));
    }

    // Surrounding whitespace and Unicode form are normalized for identity
    // checks (grilled-spec.md sec. 2).
    #[test]
    fn normalizes_unicode_form_for_duplicate_checks() {
        let precomposed = validate_name("Cafe\u{0301}").unwrap();
        let decomposed = validate_name("Café").unwrap();

        assert_eq!(precomposed.normalized, decomposed.normalized);
    }

    #[test]
    fn normalizes_case_for_duplicate_checks() {
        let lower = validate_name("animals").unwrap();
        let upper = validate_name("ANIMALS").unwrap();

        assert_eq!(lower.normalized, upper.normalized);
    }
}
