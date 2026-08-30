//! Functional core: pure decision logic with no I/O.
//!
//! Everything here is deterministic — same inputs, same outputs, no
//! files, network, clocks, or global state — and unit-tested in-file.
//! The imperative shell (`http`, `store`, `main`) does the effects and
//! calls in here for the decisions.

use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use crate::model::NewTerm;

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
}
