//! Direction-specific Progress: a Learner's knowledge state for one
//! Vocabulary Entry in one Translation Direction (grilled-spec.md sec. 2).
//! Rows are created lazily on first Answer Submission (ticket 08); this
//! ticket only needs read access to `last_correct_at` for the hard retest
//! cooldown (grilled-spec.md sec. 4).

pub mod repository;

use std::fmt;

use serde::{Deserialize, Serialize};

/// A Direction-specific Progress row's durable identity. Newtype over the
/// raw row ID (server/CODING_STANDARDS.md sec. 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct DirectionProgressId(pub i64);

impl fmt::Display for DirectionProgressId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}
