//! The box-model `Scheduler`: pure spaced-repetition logic, no I/O.
//!
//! A `Scheduler` decides two things — the starting schedule for a
//! brand-new Card, and how a Card's schedule changes after a pass or a
//! fail. Per ADR-0001 the per-Card `schedule_state` is an opaque JSON
//! string this module owns; the database never looks inside it. `now` is
//! always a parameter, so nothing here reads a clock.
//!
//! Nothing wires this in yet — ticket 03 (cards + scheduling backend) is
//! the first caller — so the module allows dead code until then.
#![allow(dead_code)]

use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::json;

/// A learner's self-graded outcome for one Card review. `pass` promotes the
/// Card a box; `fail` sends it back to the first box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rating {
    Pass,
    Fail,
}

/// Where a Card stands after a `Scheduler` decision: the opaque state blob
/// the strategy owns (ADR-0001) and the instant the Card next falls due.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Schedule {
    pub state: String,
    pub due_at: DateTime<Utc>,
}

/// Decides when a Card is next due. A pluggable strategy (ADR-0001): the
/// per-Card `state` blob is opaque JSON the implementation serialises and
/// parses itself. `now` is always passed in.
pub trait Scheduler {
    /// Starting schedule for a brand-new Card.
    fn initial_state(&self, now: DateTime<Utc>) -> Schedule;

    /// Next schedule after a review. `Err` when `state` is not a blob this
    /// strategy can read — per ADR-0001 the caller then rebuilds the Card's
    /// state by replaying its reviews.
    fn on_review(
        &self,
        state: &str,
        rating: Rating,
        now: DateTime<Utc>,
    ) -> Result<Schedule, ScheduleStateError>;
}

/// A `state` blob the active `Scheduler` could not parse.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("unreadable schedule_state: {0}")]
pub struct ScheduleStateError(String);

/// Number of Leitner boxes. Five is the classic Leitner setup — enough
/// spacing steps to be useful, few enough that a lapsed Card climbs back fast.
const BOX_COUNT: u8 = 5;

/// Days from a review until the Card is next due, per box (box 1 at index 0).
/// Roughly doubling — a passed Card's rest grows 1 → 2 → 4 → 8 → 16 days.
const BOX_INTERVAL_DAYS: [i64; BOX_COUNT as usize] = [1, 2, 4, 8, 16];

/// The v1 box-model `Scheduler` (Leitner). Its state blob is `{"box": N}`.
pub struct Leitner;

/// Leitner's private view of a `state` blob.
#[derive(Debug, Deserialize)]
struct LeitnerState {
    #[serde(rename = "box")]
    box_number: u8,
}

impl LeitnerState {
    /// A state pinned to a real box. `box_number` is clamped into
    /// `1..=BOX_COUNT` so a blob written under a differently-tuned past
    /// config still lands somewhere sensible.
    fn at(box_number: u8) -> Self {
        Self {
            box_number: box_number.clamp(1, BOX_COUNT),
        }
    }

    fn to_json(&self) -> String {
        json!({ "box": self.box_number }).to_string()
    }

    /// This box's schedule: its JSON blob plus the instant it next falls
    /// due, `BOX_INTERVAL_DAYS` days after `now`.
    fn schedule(&self, now: DateTime<Utc>) -> Schedule {
        let interval = BOX_INTERVAL_DAYS[usize::from(self.box_number - 1)];
        Schedule {
            state: self.to_json(),
            due_at: now + Duration::days(interval),
        }
    }
}

impl Scheduler for Leitner {
    fn initial_state(&self, now: DateTime<Utc>) -> Schedule {
        // A new Card sits in box 1 and is due right away.
        Schedule {
            state: LeitnerState::at(1).to_json(),
            due_at: now,
        }
    }

    fn on_review(
        &self,
        state: &str,
        rating: Rating,
        now: DateTime<Utc>,
    ) -> Result<Schedule, ScheduleStateError> {
        let current = serde_json::from_str::<LeitnerState>(state)
            .map(|s| LeitnerState::at(s.box_number).box_number)
            .map_err(|e| ScheduleStateError(e.to_string()))?;
        let next = match rating {
            Rating::Pass => (current + 1).min(BOX_COUNT),
            Rating::Fail => 1,
        };
        Ok(LeitnerState::at(next).schedule(now))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// A fixed instant — the core reads no clock, so tests pin their own.
    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap()
    }

    fn box_of(state: &str) -> u8 {
        let value: serde_json::Value = serde_json::from_str(state).unwrap();
        u8::try_from(value["box"].as_u64().unwrap()).unwrap()
    }

    #[test]
    fn new_card_starts_in_box_1_due_immediately() {
        let start = Leitner.initial_state(now());
        assert_eq!(box_of(&start.state), 1);
        assert_eq!(start.due_at, now());
    }

    #[test]
    fn pass_promotes_one_box_per_review_up_to_and_including_the_cap() {
        let mut state = Leitner.initial_state(now()).state;
        for expected in [2, 3, 4, 5, 5] {
            let next = Leitner.on_review(&state, Rating::Pass, now()).unwrap();
            assert_eq!(box_of(&next.state), expected);
            state = next.state;
        }
    }

    #[test]
    fn a_pass_at_the_top_box_does_not_overflow() {
        let top = format!(r#"{{"box":{BOX_COUNT}}}"#);
        let next = Leitner.on_review(&top, Rating::Pass, now()).unwrap();
        assert_eq!(box_of(&next.state), BOX_COUNT);
    }

    #[test]
    fn fail_resets_to_box_1_from_every_box() {
        for start in 1..=BOX_COUNT {
            let state = format!(r#"{{"box":{start}}}"#);
            let next = Leitner.on_review(&state, Rating::Fail, now()).unwrap();
            assert_eq!(box_of(&next.state), 1, "fail from box {start}");
        }
    }

    #[test]
    fn due_at_jumps_by_the_target_box_interval() {
        // A new Card is due at once.
        assert_eq!(Leitner.initial_state(now()).due_at, now());
        // pass from box 1 lands in box 2: +2 days.
        assert_eq!(
            Leitner
                .on_review(r#"{"box":1}"#, Rating::Pass, now())
                .unwrap()
                .due_at,
            now() + Duration::days(2),
        );
        // pass from box 4 lands in box 5: +16 days.
        assert_eq!(
            Leitner
                .on_review(r#"{"box":4}"#, Rating::Pass, now())
                .unwrap()
                .due_at,
            now() + Duration::days(16),
        );
        // fail from box 3 drops to box 1: due again soon, +1 day.
        assert_eq!(
            Leitner
                .on_review(r#"{"box":3}"#, Rating::Fail, now())
                .unwrap()
                .due_at,
            now() + Duration::days(1),
        );
    }

    #[test]
    fn schedule_state_round_trips_through_its_json_form() {
        let promoted = Leitner
            .on_review(r#"{"box":2}"#, Rating::Pass, now())
            .unwrap();
        assert_eq!(promoted.state, r#"{"box":3}"#);
        // The produced blob feeds straight back into the next transition.
        let reset = Leitner
            .on_review(&promoted.state, Rating::Fail, now())
            .unwrap();
        assert_eq!(reset.state, r#"{"box":1}"#);
    }

    #[test]
    fn a_blob_this_strategy_cannot_read_is_an_error() {
        assert!(Leitner.on_review("not json", Rating::Pass, now()).is_err());
        assert!(
            Leitner
                .on_review(r#"{"ease":2.5}"#, Rating::Pass, now())
                .is_err()
        );
    }

    #[test]
    fn rating_deserialises_from_its_lowercase_wire_form() {
        assert_eq!(
            serde_json::from_str::<Rating>(r#""pass""#).unwrap(),
            Rating::Pass,
        );
        assert_eq!(
            serde_json::from_str::<Rating>(r#""fail""#).unwrap(),
            Rating::Fail,
        );
    }
}
