//! `POST /api/vocabulary-entries`, `GET /api/vocabulary-entries`,
//! `GET /api/vocabulary-entries/:id`, `PATCH /api/vocabulary-entries/:id`,
//! `DELETE /api/vocabulary-entries/:id`: shared Vocabulary Entry creation,
//! listing, reading, editing, and deletion (grilled-spec.md sec. 5).

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::Utc;
use serde::Deserialize;

use crate::categories::{self, CategoryId};
use crate::http::envelope::{self, ErrorDetail, ErrorResponse, SuccessEnvelope};
use crate::http::internal_error;
use crate::state::AppState;
use crate::vocabulary_entries::repository::{
    self, EntryEdit, ListFilter, NewBulkEntry, NewEntry, RepositoryError,
};
use crate::vocabulary_entries::{
    self, EntryText, LanguageCode, VocabularyEntry, VocabularyEntryId, validate_category_ids,
    validate_language, validate_text,
};

#[derive(Debug, Deserialize)]
pub struct CreateVocabularyEntryRequest {
    pub source_language: String,
    pub source_text: String,
    pub target_language: String,
    pub target_text: String,
    pub category_ids: Vec<CategoryId>,
}

/// Creates one Vocabulary Entry from validated source/target text,
/// validated languages, and one or more Category Memberships (spec.md
/// story 13, 16, 20).
pub async fn create_vocabulary_entry(
    State(state): State<AppState>,
    Json(payload): Json<CreateVocabularyEntryRequest>,
) -> Result<(StatusCode, SuccessEnvelope<VocabularyEntry>), ErrorResponse> {
    let supported_languages = &state.config().supported_languages;

    let source_language = validate_language(&payload.source_language, supported_languages)
        .map_err(|_| validation_error("source_language", "Unsupported or empty language code"))?;
    let target_language = validate_language(&payload.target_language, supported_languages)
        .map_err(|_| validation_error("target_language", "Unsupported or empty language code"))?;
    let source_text = validate_text(&payload.source_text)
        .map_err(|_| validation_error("source_text", "Cannot be empty"))?;
    let target_text = validate_text(&payload.target_text)
        .map_err(|_| validation_error("target_text", "Cannot be empty"))?;
    validate_category_ids(&payload.category_ids)
        .map_err(|_| validation_error("category_ids", "At least one category is required"))?;

    let normalized_identity = vocabulary_entries::normalized_identity(
        &source_language,
        &source_text,
        &target_language,
        &target_text,
    );

    let entry = repository::insert(
        state.db(),
        NewEntry {
            source_language: &source_language.normalized,
            source_text: &source_text.display,
            target_language: &target_language.normalized,
            target_text: &target_text.display,
            normalized_identity: &normalized_identity,
            category_ids: &payload.category_ids,
        },
        Utc::now(),
    )
    .await
    .map_err(|error| match error {
        RepositoryError::DuplicateIdentity => conflict_error(),
        other => internal_error(other),
    })?;

    tracing::info!(vocabulary_entry_id = %entry.id, "vocabulary entry created");

    Ok((StatusCode::CREATED, envelope::success(entry, Utc::now())))
}

#[derive(Debug, Deserialize)]
pub struct BulkVocabularyEntryRow {
    pub source_language: String,
    pub source_text: String,
    pub target_language: String,
    pub target_text: String,
    pub category_name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateVocabularyEntriesBulkRequest {
    pub entries: Vec<BulkVocabularyEntryRow>,
}

struct ValidatedBulkRow {
    source_language: LanguageCode,
    source_text: EntryText,
    target_language: LanguageCode,
    target_text: EntryText,
    category_normalized_name: String,
}

/// Atomically creates multiple Vocabulary Entries from the client's parsed
/// `source | target | category` rows, resolving each row's Category by
/// normalized name. Any invalid, unknown-Category, or duplicate row —
/// including a duplicate within the same batch — rejects the whole import
/// with no partial writes (spec.md story 24, 25, 26; ticket 06).
pub async fn create_vocabulary_entries_bulk(
    State(state): State<AppState>,
    Json(payload): Json<CreateVocabularyEntriesBulkRequest>,
) -> Result<(StatusCode, SuccessEnvelope<Vec<VocabularyEntry>>), ErrorResponse> {
    let supported_languages = &state.config().supported_languages;

    if payload.entries.is_empty() {
        return Err(bulk_validation_error(0, "entries", "At least one row is required"));
    }

    let mut validated_rows = Vec::with_capacity(payload.entries.len());
    for (index, row) in payload.entries.iter().enumerate() {
        let source_language = validate_language(&row.source_language, supported_languages)
            .map_err(|_| {
                bulk_validation_error(index, "source_language", "Unsupported or empty language code")
            })?;
        let target_language = validate_language(&row.target_language, supported_languages)
            .map_err(|_| {
                bulk_validation_error(index, "target_language", "Unsupported or empty language code")
            })?;
        let source_text = validate_text(&row.source_text)
            .map_err(|_| bulk_validation_error(index, "source_text", "Cannot be empty"))?;
        let target_text = validate_text(&row.target_text)
            .map_err(|_| bulk_validation_error(index, "target_text", "Cannot be empty"))?;
        let category_name = categories::validate_name(&row.category_name)
            .map_err(|_| bulk_validation_error(index, "category_name", "Cannot be empty"))?;

        validated_rows.push(ValidatedBulkRow {
            source_language,
            source_text,
            target_language,
            target_text,
            category_normalized_name: category_name.normalized,
        });
    }

    let mut new_entries = Vec::with_capacity(validated_rows.len());
    let mut identities = Vec::with_capacity(validated_rows.len());
    for (index, row) in validated_rows.iter().enumerate() {
        let category = categories::repository::find_by_normalized_name(
            state.db(),
            &row.category_normalized_name,
        )
        .await
        .map_err(internal_error)?
        .ok_or_else(|| unknown_category_error(index))?;

        let normalized_identity = vocabulary_entries::normalized_identity(
            &row.source_language,
            &row.source_text,
            &row.target_language,
            &row.target_text,
        );

        identities.push(normalized_identity);
        new_entries.push((row, category.id));
    }

    if let Some(duplicate_index) = vocabulary_entries::find_duplicate_identity_index(&identities) {
        return Err(bulk_conflict_error(Some(duplicate_index)));
    }

    let bulk_entries: Vec<NewBulkEntry> = new_entries
        .iter()
        .zip(identities.iter())
        .map(|((row, category_id), normalized_identity)| NewBulkEntry {
            source_language: &row.source_language.normalized,
            source_text: &row.source_text.display,
            target_language: &row.target_language.normalized,
            target_text: &row.target_text.display,
            normalized_identity,
            category_id: *category_id,
        })
        .collect();

    let entries = repository::insert_bulk(state.db(), &bulk_entries, Utc::now())
        .await
        .map_err(|error| match error {
            RepositoryError::DuplicateIdentity => bulk_conflict_error(None),
            other => internal_error(other),
        })?;

    tracing::info!(count = entries.len(), "vocabulary entries bulk created");

    Ok((StatusCode::CREATED, envelope::success(entries, Utc::now())))
}

#[derive(Debug, Deserialize)]
pub struct ListVocabularyEntriesQuery {
    pub category_id: Option<CategoryId>,
    pub source_language: Option<String>,
    pub target_language: Option<String>,
}

/// Lists Vocabulary Entries, optionally filtered by Category or (unordered)
/// Language Pair.
pub async fn list_vocabulary_entries(
    State(state): State<AppState>,
    Query(query): Query<ListVocabularyEntriesQuery>,
) -> Result<SuccessEnvelope<Vec<VocabularyEntry>>, ErrorResponse> {
    let entries = repository::list(
        state.db(),
        ListFilter {
            category_id: query.category_id,
            source_language: query.source_language.map(|value| value.to_lowercase()),
            target_language: query.target_language.map(|value| value.to_lowercase()),
        },
    )
    .await
    .map_err(internal_error)?;

    Ok(envelope::success(entries, Utc::now()))
}

/// Reads a single Vocabulary Entry by ID.
pub async fn get_vocabulary_entry(
    State(state): State<AppState>,
    Path(id): Path<VocabularyEntryId>,
) -> Result<SuccessEnvelope<VocabularyEntry>, ErrorResponse> {
    let entry = repository::find_by_id(state.db(), id)
        .await
        .map_err(internal_error)?
        .ok_or_else(not_found_error)?;

    Ok(envelope::success(entry, Utc::now()))
}

#[derive(Debug, Deserialize)]
pub struct EditVocabularyEntryRequest {
    pub source_text: String,
    pub target_text: String,
    pub category_ids: Vec<CategoryId>,
    /// Accepted so a client resubmitting the read payload does not need to
    /// strip fields, but a value differing from the stored language is
    /// rejected: languages are immutable after creation (spec.md story 21).
    pub source_language: Option<String>,
    pub target_language: Option<String>,
}

/// Edits a Vocabulary Entry's text and/or Category Memberships. Source and
/// target languages are immutable after creation and rejected if a change
/// is attempted (spec.md story 19, 21).
pub async fn update_vocabulary_entry(
    State(state): State<AppState>,
    Path(id): Path<VocabularyEntryId>,
    Json(payload): Json<EditVocabularyEntryRequest>,
) -> Result<SuccessEnvelope<VocabularyEntry>, ErrorResponse> {
    let existing = repository::find_by_id(state.db(), id)
        .await
        .map_err(internal_error)?
        .ok_or_else(not_found_error)?;

    if let Some(source_language) = &payload.source_language
        && source_language.trim().to_lowercase() != existing.source_language
    {
        return Err(immutable_language_error("source_language"));
    }
    if let Some(target_language) = &payload.target_language
        && target_language.trim().to_lowercase() != existing.target_language
    {
        return Err(immutable_language_error("target_language"));
    }

    let source_text = validate_text(&payload.source_text)
        .map_err(|_| validation_error("source_text", "Cannot be empty"))?;
    let target_text = validate_text(&payload.target_text)
        .map_err(|_| validation_error("target_text", "Cannot be empty"))?;
    validate_category_ids(&payload.category_ids)
        .map_err(|_| validation_error("category_ids", "At least one category is required"))?;

    let source_language = vocabulary_entries::LanguageCode {
        normalized: existing.source_language.clone(),
    };
    let target_language = vocabulary_entries::LanguageCode {
        normalized: existing.target_language.clone(),
    };
    let normalized_identity = vocabulary_entries::normalized_identity(
        &source_language,
        &source_text,
        &target_language,
        &target_text,
    );

    let entry = repository::update(
        state.db(),
        id,
        EntryEdit {
            source_text: &source_text.display,
            target_text: &target_text.display,
            normalized_identity: &normalized_identity,
            category_ids: &payload.category_ids,
        },
        Utc::now(),
    )
    .await
    .map_err(|error| match error {
        RepositoryError::DuplicateIdentity => conflict_error(),
        other => internal_error(other),
    })?
    .ok_or_else(not_found_error)?;

    tracing::info!(vocabulary_entry_id = %entry.id, "vocabulary entry updated");

    Ok(envelope::success(entry, Utc::now()))
}

/// Deletes a Vocabulary Entry, its Category Memberships, Direction-specific
/// Progress, and practice history in one transaction (spec.md story 22,
/// 23).
pub async fn delete_vocabulary_entry(
    State(state): State<AppState>,
    Path(id): Path<VocabularyEntryId>,
) -> Result<SuccessEnvelope<()>, ErrorResponse> {
    let deleted = repository::delete(state.db(), id)
        .await
        .map_err(internal_error)?;

    if !deleted {
        return Err(not_found_error());
    }

    tracing::info!(vocabulary_entry_id = %id, "vocabulary entry deleted");

    Ok(envelope::success_without_data(Utc::now()))
}

fn validation_error(field: &str, reason: &str) -> ErrorResponse {
    ErrorResponse {
        status_code: StatusCode::BAD_REQUEST,
        envelope: envelope::error(
            "VALIDATION_ERROR",
            "Invalid vocabulary entry.",
            vec![ErrorDetail {
                field: field.to_string(),
                reason: reason.to_string(),
            }],
            Utc::now(),
        ),
    }
}

fn bulk_validation_error(index: usize, field: &str, reason: &str) -> ErrorResponse {
    ErrorResponse {
        status_code: StatusCode::BAD_REQUEST,
        envelope: envelope::error(
            "VALIDATION_ERROR",
            "Invalid vocabulary entry in bulk import.",
            vec![ErrorDetail {
                field: format!("entries[{index}].{field}"),
                reason: reason.to_string(),
            }],
            Utc::now(),
        ),
    }
}

fn unknown_category_error(index: usize) -> ErrorResponse {
    ErrorResponse {
        status_code: StatusCode::BAD_REQUEST,
        envelope: envelope::error(
            "UNKNOWN_CATEGORY",
            "Bulk import references a category that does not exist.",
            vec![ErrorDetail {
                field: format!("entries[{index}].category_name"),
                reason: "No category with this name exists".to_string(),
            }],
            Utc::now(),
        ),
    }
}

fn bulk_conflict_error(index: Option<usize>) -> ErrorResponse {
    let field = match index {
        Some(index) => format!("entries[{index}].target_text"),
        None => "target_text".to_string(),
    };
    ErrorResponse {
        status_code: StatusCode::CONFLICT,
        envelope: envelope::error(
            "VOCABULARY_ENTRY_CONFLICT",
            "A vocabulary entry with this language-and-text pair already exists.",
            vec![ErrorDetail {
                field,
                reason: "Already in use".to_string(),
            }],
            Utc::now(),
        ),
    }
}

fn immutable_language_error(field: &str) -> ErrorResponse {
    ErrorResponse {
        status_code: StatusCode::BAD_REQUEST,
        envelope: envelope::error(
            "VALIDATION_ERROR",
            "Vocabulary entry languages cannot be changed after creation.",
            vec![ErrorDetail {
                field: field.to_string(),
                reason: "Languages are immutable".to_string(),
            }],
            Utc::now(),
        ),
    }
}

fn not_found_error() -> ErrorResponse {
    ErrorResponse {
        status_code: StatusCode::NOT_FOUND,
        envelope: envelope::error(
            "VOCABULARY_ENTRY_NOT_FOUND",
            "That vocabulary entry no longer exists.",
            vec![ErrorDetail {
                field: "id".to_string(),
                reason: "Not found".to_string(),
            }],
            Utc::now(),
        ),
    }
}

fn conflict_error() -> ErrorResponse {
    ErrorResponse {
        status_code: StatusCode::CONFLICT,
        envelope: envelope::error(
            "VOCABULARY_ENTRY_CONFLICT",
            "A vocabulary entry with this language-and-text pair already exists.",
            vec![ErrorDetail {
                field: "target_text".to_string(),
                reason: "Already in use".to_string(),
            }],
            Utc::now(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_error_returns_400() {
        let error = validation_error("source_text", "Cannot be empty");

        assert_eq!(error.status_code, StatusCode::BAD_REQUEST);
        assert_eq!(error.envelope.error.code, "VALIDATION_ERROR");
        assert_eq!(error.envelope.error.details[0].field, "source_text");
    }

    #[test]
    fn not_found_error_returns_404() {
        let error = not_found_error();

        assert_eq!(error.status_code, StatusCode::NOT_FOUND);
        assert_eq!(error.envelope.error.code, "VOCABULARY_ENTRY_NOT_FOUND");
    }

    #[test]
    fn conflict_error_returns_409() {
        let error = conflict_error();

        assert_eq!(error.status_code, StatusCode::CONFLICT);
        assert_eq!(error.envelope.error.code, "VOCABULARY_ENTRY_CONFLICT");
    }
}
