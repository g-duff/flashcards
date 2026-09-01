//! Functional core: pure decision logic with no I/O.
//!
//! Everything here is deterministic — same inputs, same outputs, no
//! files, network, clocks, or global state — and unit-tested in-file.
//! The imperative shell (`http`, `store`, `main`) does the effects and
//! calls in here for the decisions.

pub mod scheduler;

use chrono::{DateTime, Utc};
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use crate::model::{NewTerm, PromptSide};

// Re-exported so callers reach the scheduling seam as `core::Scheduler`
// (per `docs/DESIGN.md`), while the impl keeps its own file per the
// module-layout standard.
pub use scheduler::{Leitner, Schedule, Scheduler};

/// The longest a card side may be. Arbitrary but generous — a flashcard
/// is a prompt, not an essay.
pub const MAX_SIDE_LEN: usize = 1_000;

/// The one hardcoded namespace UUID every derived id in this app hashes
/// under. Generated once (a random v4) and frozen — changing it would
/// change every Term id. See `docs/DESIGN.md` § Identifiers.
pub const APP_NS: Uuid = Uuid::from_u128(0x1b67_1a64_40d5_491e_99b0_da01_ff1f_3341);

/// ASCII unit separator (`\x1f`). Joins the identity fields in the
/// canonical string; it cannot occur in normal vocabulary text, so the
/// join is unambiguous.
const UNIT_SEP: &str = "\u{1f}";

/// Why a `NewTerm` was rejected: the named identity field was empty after
/// trimming. Carries the field name so the shell renders the message
/// without matching on a variant.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("{0} must not be empty")]
pub struct EmptyField(pub &'static str);

/// Why an import batch was rejected: element `index` failed the same
/// validation as `POST /terms`. Carries the index so the shell's `400`
/// body names the offending element and imports nothing (all-or-nothing).
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("element {index}: {source}")]
pub struct ImportElementError {
    pub index: usize,
    pub source: EmptyField,
}

/// The canonical string a Term's id hashes over: the three identity
/// fields, each NFC-normalised and outer-trimmed, joined by the unit
/// separator. Case is preserved (`él` and `el` are different words) and
/// `notes` is excluded — it is not part of a Term's identity.
pub fn canonical_name(term: &NewTerm) -> String {
    [
        term.foreign_lang.as_str(),
        term.foreign_text.as_str(),
        term.pivot_text.as_str(),
    ]
    .into_iter()
    .map(|field| field.trim().nfc().collect::<String>())
    .collect::<Vec<_>>()
    .join(UNIT_SEP)
}

/// The deterministic UUIDv5 id for a Term, derived from its
/// [`canonical_name`] under [`APP_NS`]. The same three texts always
/// produce the same id — this is what makes import idempotent.
pub fn term_id(term: &NewTerm) -> Uuid {
    Uuid::new_v5(&APP_NS, canonical_name(term).as_bytes())
}

/// The deterministic UUIDv5 id for a Card: hashed over
/// `term_id ␟ prompt_side` under [`APP_NS`]. A Term's two Cards therefore
/// have stable, distinct ids, so creating them is idempotent just like
/// the Term itself.
pub fn card_id(term_id: &str, prompt_side: PromptSide) -> Uuid {
    let canonical = [term_id, prompt_side.as_str()].join(UNIT_SEP);
    Uuid::new_v5(&APP_NS, canonical.as_bytes())
}

/// Resolve the `(prompt, answer)` a Card shows from its Term's two texts.
/// `Foreign` prompts with the foreign text and expects the pivot text;
/// `Pivot` is the reverse.
pub fn prompt_and_answer(
    prompt_side: PromptSide,
    foreign_text: &str,
    pivot_text: &str,
) -> (String, String) {
    match prompt_side {
        PromptSide::Foreign => (foreign_text.to_string(), pivot_text.to_string()),
        PromptSide::Pivot => (pivot_text.to_string(), foreign_text.to_string()),
    }
}

/// A Card ready to be inserted alongside its Term: the derived id, the
/// side it prompts, and its starting schedule from the active
/// [`Scheduler`].
#[derive(Clone, Debug)]
pub struct CardSeed {
    pub id: String,
    pub prompt_side: PromptSide,
    pub schedule: Schedule,
}

/// The two Cards a Term yields — recognition (`Foreign`) and production
/// (`Pivot`) — each with its id derived from `term_id` and its initial
/// schedule from `scheduler`. This is the one place the pair is built, so
/// `POST /terms` and `POST /terms/import` (ticket 05) stay in step.
pub fn card_seeds(term_id: &str, scheduler: &dyn Scheduler, now: DateTime<Utc>) -> [CardSeed; 2] {
    PromptSide::ALL.map(|prompt_side| CardSeed {
        id: card_id(term_id, prompt_side).to_string(),
        prompt_side,
        schedule: scheduler.initial_state(now),
    })
}

/// Validate a new Term: each of the three text fields must be non-empty
/// after trimming. `notes` is free-form and never checked.
pub fn validate_new_term(term: &NewTerm) -> Result<(), EmptyField> {
    let identity_fields = [
        ("foreign_lang", term.foreign_lang.as_str()),
        ("foreign_text", term.foreign_text.as_str()),
        ("pivot_text", term.pivot_text.as_str()),
    ];
    match identity_fields
        .into_iter()
        .find(|(_, value)| value.trim().is_empty())
    {
        Some((name, _)) => Err(EmptyField(name)),
        None => Ok(()),
    }
}

/// Validate every element of an import batch in order, each by the same
/// rules as `POST /terms`. `Err` on the first invalid element, naming its
/// index — the shell then imports nothing (all-or-nothing parse/validate).
pub fn validate_import(terms: &[NewTerm]) -> Result<(), ImportElementError> {
    terms.iter().enumerate().try_for_each(|(index, term)| {
        validate_new_term(term).map_err(|source| ImportElementError { index, source })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn term(foreign_lang: &str, foreign_text: &str, pivot_text: &str) -> NewTerm {
        NewTerm {
            foreign_lang: foreign_lang.to_string(),
            foreign_text: foreign_text.to_string(),
            pivot_text: pivot_text.to_string(),
            notes: None,
        }
    }

    #[test]
    fn canonical_name_joins_trimmed_fields_with_the_unit_separator() {
        let got = canonical_name(&term("  es ", " perro", "dog "));
        assert_eq!(got, "es\u{1f}perro\u{1f}dog");
    }

    #[test]
    fn canonical_name_preserves_case() {
        assert_ne!(
            canonical_name(&term("es", "Él", "he")),
            canonical_name(&term("es", "él", "he")),
        );
    }

    #[test]
    fn canonical_name_ignores_notes() {
        let mut a = term("es", "perro", "dog");
        a.notes = Some("el perro (m)".to_string());
        let b = term("es", "perro", "dog");
        assert_eq!(canonical_name(&a), canonical_name(&b));
    }

    #[test]
    fn canonical_name_normalises_to_nfc() {
        // "é" as base 'e' + combining acute (NFD) vs precomposed (NFC).
        let decomposed = term("es", "cafe\u{301}", "coffee");
        let composed = term("es", "caf\u{e9}", "coffee");
        assert_eq!(canonical_name(&decomposed), canonical_name(&composed));
    }

    #[test]
    fn term_id_is_stable_for_the_same_texts() {
        assert_eq!(
            term_id(&term("es", "perro", "dog")),
            term_id(&term(" es ", " perro ", " dog ")),
        );
    }

    #[test]
    fn term_id_differs_when_any_identity_field_differs() {
        let base = term_id(&term("es", "perro", "dog"));
        assert_ne!(base, term_id(&term("fr", "perro", "dog")));
        assert_ne!(base, term_id(&term("es", "gato", "dog")));
        assert_ne!(base, term_id(&term("es", "perro", "cat")));
    }

    #[test]
    fn term_id_is_a_v5_uuid() {
        assert_eq!(
            term_id(&term("es", "perro", "dog")).get_version(),
            Some(uuid::Version::Sha1),
        );
    }

    #[test]
    fn card_id_is_stable_for_the_same_term_and_side() {
        let term = "9f1c1c8a-1b2d-5e3f-8a4b-6c7d8e9f0a1b";
        assert_eq!(
            card_id(term, PromptSide::Foreign),
            card_id(term, PromptSide::Foreign),
        );
    }

    #[test]
    fn card_id_differs_by_side_and_by_term() {
        let term = "9f1c1c8a-1b2d-5e3f-8a4b-6c7d8e9f0a1b";
        assert_ne!(
            card_id(term, PromptSide::Foreign),
            card_id(term, PromptSide::Pivot),
        );
        assert_ne!(
            card_id(term, PromptSide::Foreign),
            card_id("another-term", PromptSide::Foreign),
        );
    }

    #[test]
    fn card_id_is_a_v5_uuid() {
        assert_eq!(
            card_id("t", PromptSide::Pivot).get_version(),
            Some(uuid::Version::Sha1),
        );
    }

    #[test]
    fn prompt_and_answer_follows_the_prompt_side() {
        assert_eq!(
            prompt_and_answer(PromptSide::Foreign, "gato", "cat"),
            ("gato".to_string(), "cat".to_string()),
        );
        assert_eq!(
            prompt_and_answer(PromptSide::Pivot, "gato", "cat"),
            ("cat".to_string(), "gato".to_string()),
        );
    }

    #[test]
    fn card_seeds_builds_one_card_per_side_each_in_box_1_due_now() {
        let now = chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 1, 1, 12, 0, 0).unwrap();
        let seeds = card_seeds("term-1", &Leitner, now);

        assert_eq!(seeds[0].prompt_side, PromptSide::Foreign);
        assert_eq!(seeds[1].prompt_side, PromptSide::Pivot);
        assert_ne!(seeds[0].id, seeds[1].id);
        for seed in &seeds {
            assert_eq!(seed.schedule.state, r#"{"box":1}"#);
            assert_eq!(seed.schedule.due_at, now);
        }
    }

    #[test]
    fn validate_accepts_a_normal_term() {
        assert_eq!(validate_new_term(&term("es", "perro", "dog")), Ok(()));
    }

    #[test]
    fn validate_rejects_each_blank_text_field() {
        assert_eq!(
            validate_new_term(&term("  ", "perro", "dog")),
            Err(EmptyField("foreign_lang")),
        );
        assert_eq!(
            validate_new_term(&term("es", "\t\n", "dog")),
            Err(EmptyField("foreign_text")),
        );
        assert_eq!(
            validate_new_term(&term("es", "perro", "")),
            Err(EmptyField("pivot_text")),
        );
    }

    #[test]
    fn empty_field_error_message_names_the_field() {
        assert_eq!(
            EmptyField("foreign_text").to_string(),
            "foreign_text must not be empty",
        );
    }

    #[test]
    fn validate_ignores_notes() {
        let blank_notes = NewTerm {
            notes: Some(String::new()),
            ..term("es", "perro", "dog")
        };
        assert_eq!(validate_new_term(&blank_notes), Ok(()));
    }

    #[test]
    fn validate_import_accepts_a_batch_of_good_terms() {
        let batch = [term("es", "perro", "dog"), term("es", "gato", "cat")];
        assert_eq!(validate_import(&batch), Ok(()));
    }

    #[test]
    fn validate_import_accepts_an_empty_batch() {
        assert_eq!(validate_import(&[]), Ok(()));
    }

    #[test]
    fn validate_import_reports_the_first_bad_element_by_index() {
        let batch = [
            term("es", "perro", "dog"),
            term("es", "", "cat"),
            term("es", "pato", ""),
        ];
        assert_eq!(
            validate_import(&batch),
            Err(ImportElementError {
                index: 1,
                source: EmptyField("foreign_text"),
            }),
        );
    }

    #[test]
    fn import_element_error_message_names_the_index_and_field() {
        let err = ImportElementError {
            index: 3,
            source: EmptyField("pivot_text"),
        };
        assert_eq!(err.to_string(), "element 3: pivot_text must not be empty");
    }
}
