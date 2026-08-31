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

/// Which of a Term's two texts a Card prompts with. `foreign` shows the
/// foreign text and asks for the pivot text (recognition); `pivot` is the
/// reverse (production). Serialises as its lowercase name, matching the
/// `card.prompt_side` CHECK constraint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptSide {
    Foreign,
    Pivot,
}

impl PromptSide {
    /// Both sides, in the order a Term's Cards are created.
    pub const ALL: [PromptSide; 2] = [PromptSide::Foreign, PromptSide::Pivot];

    /// The wire / storage token for this side.
    pub fn as_str(self) -> &'static str {
        match self {
            PromptSide::Foreign => "foreign",
            PromptSide::Pivot => "pivot",
        }
    }
}

impl std::str::FromStr for PromptSide {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "foreign" => Ok(PromptSide::Foreign),
            "pivot" => Ok(PromptSide::Pivot),
            other => Err(format!("unknown prompt_side: {other}")),
        }
    }
}

/// A Card with its Term's text resolved for display during a practice
/// run: `prompt` is the side shown, `answer` the side to recall, both
/// picked by `prompt_side`. `box_number` is the current Leitner box,
/// read from the Card's opaque `schedule_state`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PracticeCard {
    pub id: String,
    pub term_id: String,
    pub prompt_side: PromptSide,
    pub prompt: String,
    pub answer: String,
    pub notes: Option<String>,
    pub due_at: String,
    #[serde(rename = "box")]
    pub box_number: u8,
}

/// A learner's self-graded outcome for one Card review. `pass` promotes
/// the Card a box; `fail` sends it back to the first box. Serialises as
/// its lowercase name, matching the `review.rating` CHECK constraint. The
/// vocabulary leaves room for a finer scale later.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rating {
    Pass,
    Fail,
}

impl Rating {
    /// The wire / storage token for this rating (the `review.rating`
    /// CHECK values).
    pub fn as_str(self) -> &'static str {
        match self {
            Rating::Pass => "pass",
            Rating::Fail => "fail",
        }
    }
}

/// The body of `POST /cards/{id}/reviews` — the learner's self-graded
/// outcome for one attempt at a Card.
#[derive(Clone, Debug, Deserialize)]
pub struct NewReview {
    pub rating: Rating,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_side_round_trips_through_its_wire_token() {
        for side in PromptSide::ALL {
            assert_eq!(side.as_str().parse::<PromptSide>(), Ok(side));
        }
        assert!("sideways".parse::<PromptSide>().is_err());
    }

    #[test]
    fn rating_as_str_matches_the_wire_form() {
        assert_eq!(Rating::Pass.as_str(), "pass");
        assert_eq!(Rating::Fail.as_str(), "fail");
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
