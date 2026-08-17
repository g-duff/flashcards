//! Vocabulary Entry domain: a language-neutral translation pair shared by
//! every Learner (grilled-spec.md sec. 2).

pub mod repository;

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::categories::CategoryId;

/// A Vocabulary Entry's durable identity. Newtype over the raw row ID so a
/// caller cannot accidentally pass an unrelated ID where a Vocabulary Entry
/// ID is expected (server/CODING_STANDARDS.md sec. 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct VocabularyEntryId(pub i64);

impl fmt::Display for VocabularyEntryId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VocabularyEntry {
    pub id: VocabularyEntryId,
    pub source_language: String,
    pub source_text: String,
    pub target_language: String,
    pub target_text: String,
    pub category_ids: Vec<CategoryId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Vocabulary Entry text (source or target) that has passed validation:
/// trimmed and non-empty.
#[derive(Debug, Clone, PartialEq)]
pub struct EntryText {
    pub display: String,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EntryTextError {
    #[error("text must not be empty")]
    Empty,
}

/// Validates and normalizes raw Vocabulary Entry text. Trims surrounding
/// whitespace and rejects an empty result. Case, punctuation, and accents
/// are preserved as meaningful content (grilled-spec.md sec. 2).
pub fn validate_text(raw: &str) -> Result<EntryText, EntryTextError> {
    let display = raw.trim().to_string();
    if display.is_empty() {
        return Err(EntryTextError::Empty);
    }
    Ok(EntryText { display })
}

/// A language code that has passed validation: normalized to lowercase and
/// checked against the configured supported-language list (spec.md story
/// 20).
#[derive(Debug, Clone, PartialEq)]
pub struct LanguageCode {
    pub normalized: String,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LanguageCodeError {
    #[error("language code must not be empty")]
    Empty,
    #[error("language code is not supported")]
    Unsupported,
}

/// Normalizes a raw language code to lowercase and validates it against the
/// configured supported-language list.
pub fn validate_language(
    raw: &str,
    supported_languages: &[String],
) -> Result<LanguageCode, LanguageCodeError> {
    let normalized = raw.trim().to_lowercase();
    if normalized.is_empty() {
        return Err(LanguageCodeError::Empty);
    }
    if !supported_languages
        .iter()
        .any(|supported| supported.to_lowercase() == normalized)
    {
        return Err(LanguageCodeError::Unsupported);
    }
    Ok(LanguageCode { normalized })
}

/// Derives the normalized global-identity key used to reject duplicate
/// language-and-text pairs (spec.md story 15). Unicode NFC + trim
/// normalization is applied to text; case, punctuation, and accents remain
/// meaningful, so text is not lowercased. Language codes are already
/// normalized to lowercase.
pub fn normalized_identity(
    source_language: &LanguageCode,
    source_text: &EntryText,
    target_language: &LanguageCode,
    target_text: &EntryText,
) -> String {
    format!(
        "{}\u{1}{}\u{1}{}\u{1}{}",
        source_language.normalized,
        source_text.display.nfc().collect::<String>(),
        target_language.normalized,
        target_text.display.nfc().collect::<String>(),
    )
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CategoryIdsError {
    #[error("at least one category is required")]
    Empty,
}

/// Validates that a Vocabulary Entry has at least one Category Membership
/// (spec.md story 16).
pub fn validate_category_ids(category_ids: &[CategoryId]) -> Result<(), CategoryIdsError> {
    if category_ids.is_empty() {
        return Err(CategoryIdsError::Empty);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supported() -> Vec<String> {
        vec!["en".to_string(), "es".to_string()]
    }

    #[test]
    fn trims_whitespace_and_preserves_text_case() {
        let text = validate_text("  manzana  ").unwrap();

        assert_eq!(text.display, "manzana");
    }

    #[test]
    fn rejects_empty_text_after_trimming() {
        assert_eq!(validate_text("   "), Err(EntryTextError::Empty));
    }

    // Language codes are normalized to lowercase ISO 639-1 and validated
    // against the configured supported-language list (spec.md story 20).
    #[test]
    fn normalizes_language_code_to_lowercase() {
        let code = validate_language("ES", &supported()).unwrap();

        assert_eq!(code.normalized, "es");
    }

    #[test]
    fn rejects_unsupported_language_code() {
        assert_eq!(
            validate_language("fr", &supported()),
            Err(LanguageCodeError::Unsupported)
        );
    }

    #[test]
    fn rejects_empty_language_code() {
        assert_eq!(
            validate_language("   ", &supported()),
            Err(LanguageCodeError::Empty)
        );
    }

    // Surrounding whitespace and Unicode form are normalized for duplicate
    // identity, while case, punctuation, and accents remain meaningful
    // (grilled-spec.md sec. 2; spec.md story 15).
    #[test]
    fn normalized_identity_matches_across_unicode_forms() {
        let source_language = validate_language("es", &supported()).unwrap();
        let target_language = validate_language("en", &supported()).unwrap();
        let precomposed = validate_text("Jose\u{0301}").unwrap();
        let decomposed = validate_text("José").unwrap();

        let identity_a = normalized_identity(
            &source_language,
            &precomposed,
            &target_language,
            &precomposed,
        );
        let identity_b =
            normalized_identity(&source_language, &decomposed, &target_language, &decomposed);

        assert_eq!(identity_a, identity_b);
    }

    #[test]
    fn normalized_identity_treats_case_as_meaningful() {
        let source_language = validate_language("es", &supported()).unwrap();
        let target_language = validate_language("en", &supported()).unwrap();
        let lower = validate_text("manzana").unwrap();
        let upper = validate_text("Manzana").unwrap();

        let identity_lower =
            normalized_identity(&source_language, &lower, &target_language, &lower);
        let identity_upper =
            normalized_identity(&source_language, &upper, &target_language, &upper);

        assert_ne!(identity_lower, identity_upper);
    }

    #[test]
    fn rejects_zero_category_ids() {
        assert_eq!(validate_category_ids(&[]), Err(CategoryIdsError::Empty));
    }

    #[test]
    fn accepts_one_or_more_category_ids() {
        assert_eq!(validate_category_ids(&[CategoryId(1)]), Ok(()));
    }
}
