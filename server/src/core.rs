//! Functional core: pure decision logic with no I/O.
//!
//! Everything here is deterministic — same inputs, same outputs, no
//! files, network, clocks, or global state — and unit-tested in-file.
//! The imperative shell (`http`, `store`, `main`) does the effects and
//! calls in here for the decisions.

pub mod scheduler;

// Re-exported so callers reach the scheduling seam as `core::Scheduler`
// (per `docs/DESIGN.md`), while the impl keeps its own file per the
// module-layout standard. Unused until ticket 03 wires it in.
#[allow(unused_imports)]
pub use scheduler::{Leitner, Rating, Schedule, ScheduleStateError, Scheduler};

/// The longest a card side may be. Arbitrary but generous — a flashcard
/// is a prompt, not an essay.
pub const MAX_SIDE_LEN: usize = 1_000;

/// Why a `POST /cards` body was rejected. One variant per rule so the
/// shell can map each to a message without string-matching.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum CardError {
    #[error("front must not be empty")]
    EmptyFront,
    #[error("back must not be empty")]
    EmptyBack,
    #[error("front must be at most {MAX_SIDE_LEN} characters")]
    FrontTooLong,
    #[error("back must be at most {MAX_SIDE_LEN} characters")]
    BackTooLong,
}

/// Validate the two sides of a new card. Whitespace-only is empty; length
/// is counted in characters, not bytes.
pub fn validate_new_card(front: &str, back: &str) -> Result<(), CardError> {
    if front.trim().is_empty() {
        return Err(CardError::EmptyFront);
    }
    if back.trim().is_empty() {
        return Err(CardError::EmptyBack);
    }
    if front.chars().count() > MAX_SIDE_LEN {
        return Err(CardError::FrontTooLong);
    }
    if back.chars().count() > MAX_SIDE_LEN {
        return Err(CardError::BackTooLong);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_normal_card() {
        assert_eq!(validate_new_card("capital of France", "Paris"), Ok(()));
    }

    #[test]
    fn rejects_blank_sides() {
        assert_eq!(validate_new_card("   ", "Paris"), Err(CardError::EmptyFront));
        assert_eq!(validate_new_card("q", "\t\n"), Err(CardError::EmptyBack));
    }

    #[test]
    fn rejects_overlong_sides() {
        let long = "x".repeat(MAX_SIDE_LEN + 1);
        assert_eq!(
            validate_new_card(&long, "ok"),
            Err(CardError::FrontTooLong)
        );
        assert_eq!(validate_new_card("ok", &long), Err(CardError::BackTooLong));
    }

    #[test]
    fn length_is_counted_in_chars_not_bytes() {
        let multibyte = "é".repeat(MAX_SIDE_LEN);
        assert_eq!(validate_new_card(&multibyte, "ok"), Ok(()));
    }
}
