//! Wire types shared between the HTTP layer and the store. Plain data,
//! no behaviour.

use serde::{Deserialize, Serialize};

/// A single flashcard. `id` is assigned by the store on insert.
#[derive(Clone, Debug, Serialize)]
pub struct Card {
    pub id: u64,
    pub front: String,
    pub back: String,
}

/// The body of `POST /cards`. Validated by [`crate::core::validate_new_card`]
/// before it reaches the store.
#[derive(Debug, Deserialize)]
pub struct NewCard {
    pub front: String,
    pub back: String,
}
