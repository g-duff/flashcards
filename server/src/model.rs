//! Wire types shared between the HTTP layer and the store. Plain data,
//! no behaviour.

use serde::{Deserialize, Serialize};

/// A vocabulary pair as stored. `id` is the deterministic UUIDv5 over the
/// three text fields (see [`crate::core::term_id`]); `created_at` is an
/// ISO-8601 timestamp. The three texts are immutable identity — only
/// `notes` is ever edited.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Term {
    pub id: String,
    pub foreign_lang: String,
    pub foreign_text: String,
    pub pivot_text: String,
    pub notes: Option<String>,
    pub created_at: String,
}

/// The body of `POST /terms`. Validated by
/// [`crate::core::validate_new_term`] before it reaches the store.
#[derive(Clone, Debug, Deserialize)]
pub struct NewTerm {
    pub foreign_lang: String,
    pub foreign_text: String,
    pub pivot_text: String,
    #[serde(default)]
    pub notes: Option<String>,
}

/// The body of `PATCH /terms/{id}` — the only mutable field of a Term.
/// An explicit `null` clears the notes.
#[derive(Clone, Debug, Deserialize)]
pub struct NotesPatch {
    pub notes: Option<String>,
}

/// The body of a successful `DELETE /terms/{id}` (the `Deleted` schema in
/// `openapi.yaml`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Deleted {
    pub deleted: String,
}
