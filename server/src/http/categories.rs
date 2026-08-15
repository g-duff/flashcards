//! `POST /api/categories`, `GET /api/categories`, `GET /api/categories/:id`,
//! `PATCH /api/categories/:id`, `DELETE /api/categories/:id`: shared
//! Category creation, listing, reading, renaming, and deletion
//! (grilled-spec.md sec. 5).

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::Utc;
use serde::Deserialize;

use crate::categories::repository::{self, RepositoryError};
use crate::categories::{self, Category, CategoryId, CategorySort};
use crate::http::envelope::{self, ErrorDetail, ErrorResponse, SuccessEnvelope};
use crate::http::internal_error;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
}

/// Creates a shared Category with a unique display name (spec.md story 8).
pub async fn create_category(
    State(state): State<AppState>,
    Json(payload): Json<CreateCategoryRequest>,
) -> Result<(StatusCode, SuccessEnvelope<Category>), ErrorResponse> {
    let name = categories::validate_name(&payload.name).map_err(|_| validation_error())?;

    let category = repository::insert(state.db(), &name, Utc::now())
        .await
        .map_err(|error| match error {
            RepositoryError::DuplicateName => conflict_error(),
            other => internal_error(other),
        })?;

    tracing::info!(category_id = %category.id, name = %category.name, "category created");

    Ok((StatusCode::CREATED, envelope::success(category, Utc::now())))
}

#[derive(Debug, Deserialize)]
pub struct ListCategoriesQuery {
    pub sort: Option<CategorySort>,
}

/// Lists every Category, sortable by creation date (default) or
/// alphabetically (spec.md story 9).
pub async fn list_categories(
    State(state): State<AppState>,
    Query(query): Query<ListCategoriesQuery>,
) -> Result<SuccessEnvelope<Vec<Category>>, ErrorResponse> {
    let sort = query.sort.unwrap_or(CategorySort::CreatedAt);

    let categories = repository::list(state.db(), sort)
        .await
        .map_err(internal_error)?;

    Ok(envelope::success(categories, Utc::now()))
}

/// Reads a single Category by ID.
pub async fn get_category(
    State(state): State<AppState>,
    Path(id): Path<CategoryId>,
) -> Result<SuccessEnvelope<Category>, ErrorResponse> {
    let category = repository::find_by_id(state.db(), id)
        .await
        .map_err(internal_error)?
        .ok_or_else(not_found_error)?;

    Ok(envelope::success(category, Utc::now()))
}

#[derive(Debug, Deserialize)]
pub struct RenameCategoryRequest {
    pub name: String,
}

/// Renames a Category, preserving its durable ID (spec.md story 10). The
/// new name is subject to the same case-insensitive, trimmed uniqueness
/// check as creation.
pub async fn rename_category(
    State(state): State<AppState>,
    Path(id): Path<CategoryId>,
    Json(payload): Json<RenameCategoryRequest>,
) -> Result<SuccessEnvelope<Category>, ErrorResponse> {
    let name = categories::validate_name(&payload.name).map_err(|_| validation_error())?;

    let category = repository::rename(state.db(), id, &name, Utc::now())
        .await
        .map_err(|error| match error {
            RepositoryError::DuplicateName => conflict_error(),
            other => internal_error(other),
        })?
        .ok_or_else(not_found_error)?;

    tracing::info!(category_id = %category.id, name = %category.name, "category renamed");

    Ok(envelope::success(category, Utc::now()))
}

/// Deletes a Category. Never cascade-deletes Vocabulary Entries; rejection
/// of unsafe deletion (orphaning a Vocabulary Entry) is added alongside
/// Category Memberships in ticket 05 (spec.md story 11, 12).
pub async fn delete_category(
    State(state): State<AppState>,
    Path(id): Path<CategoryId>,
) -> Result<SuccessEnvelope<()>, ErrorResponse> {
    let deleted = repository::delete(state.db(), id)
        .await
        .map_err(internal_error)?;

    if !deleted {
        return Err(not_found_error());
    }

    tracing::info!(category_id = %id, "category deleted");

    Ok(envelope::success_without_data(Utc::now()))
}

fn validation_error() -> ErrorResponse {
    ErrorResponse {
        status_code: StatusCode::BAD_REQUEST,
        envelope: envelope::error(
            "VALIDATION_ERROR",
            "Category name cannot be empty.",
            vec![ErrorDetail {
                field: "name".to_string(),
                reason: "Cannot be empty".to_string(),
            }],
            Utc::now(),
        ),
    }
}

fn not_found_error() -> ErrorResponse {
    ErrorResponse {
        status_code: StatusCode::NOT_FOUND,
        envelope: envelope::error(
            "CATEGORY_NOT_FOUND",
            "That category no longer exists.",
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
            "CATEGORY_NAME_CONFLICT",
            "A category with this name already exists.",
            vec![ErrorDetail {
                field: "name".to_string(),
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
    fn validation_error_returns_400_with_correct_error_code() {
        let error = validation_error();

        assert_eq!(error.status_code, StatusCode::BAD_REQUEST);
        assert_eq!(error.envelope.error.code, "VALIDATION_ERROR");
        assert_eq!(
            error.envelope.error.message,
            "Category name cannot be empty."
        );
        assert_eq!(error.envelope.error.details.len(), 1);
        assert_eq!(error.envelope.error.details[0].field, "name");
        assert_eq!(error.envelope.error.details[0].reason, "Cannot be empty");
    }

    #[test]
    fn not_found_error_returns_404_with_correct_error_code() {
        let error = not_found_error();

        assert_eq!(error.status_code, StatusCode::NOT_FOUND);
        assert_eq!(error.envelope.error.code, "CATEGORY_NOT_FOUND");
        assert_eq!(
            error.envelope.error.message,
            "That category no longer exists."
        );
        assert_eq!(error.envelope.error.details.len(), 1);
        assert_eq!(error.envelope.error.details[0].field, "id");
        assert_eq!(error.envelope.error.details[0].reason, "Not found");
    }

    #[test]
    fn conflict_error_returns_409_with_correct_error_code() {
        let error = conflict_error();

        assert_eq!(error.status_code, StatusCode::CONFLICT);
        assert_eq!(error.envelope.error.code, "CATEGORY_NAME_CONFLICT");
        assert_eq!(
            error.envelope.error.message,
            "A category with this name already exists."
        );
        assert_eq!(error.envelope.error.details.len(), 1);
        assert_eq!(error.envelope.error.details[0].field, "name");
        assert_eq!(error.envelope.error.details[0].reason, "Already in use");
    }
}
