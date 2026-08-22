//! Translation Direction: which side of a Vocabulary Entry is the prompt
//! and which is the answer for a given Practice Session (grilled-spec.md
//! sec. 2).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::vocabulary_entries::VocabularyEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationDirection {
    SourceToTarget,
    TargetToSource,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("\"{0}\" is not a recognized translation direction")]
pub struct ParseTranslationDirectionError(String);

impl fmt::Display for TranslationDirection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let raw = match self {
            Self::SourceToTarget => "source_to_target",
            Self::TargetToSource => "target_to_source",
        };
        write!(formatter, "{raw}")
    }
}

impl FromStr for TranslationDirection {
    type Err = ParseTranslationDirectionError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw {
            "source_to_target" => Ok(Self::SourceToTarget),
            "target_to_source" => Ok(Self::TargetToSource),
            other => Err(ParseTranslationDirectionError(other.to_string())),
        }
    }
}

impl TranslationDirection {
    /// The prompt text shown to the Learner: the entry's source side for
    /// `SourceToTarget`, its target side for `TargetToSource`.
    pub fn prompt_text(self, entry: &VocabularyEntry) -> &str {
        match self {
            Self::SourceToTarget => &entry.source_text,
            Self::TargetToSource => &entry.target_text,
        }
    }

    /// The correct answer text: the opposite side of [`prompt_text`].
    pub fn answer_text(self, entry: &VocabularyEntry) -> &str {
        match self {
            Self::SourceToTarget => &entry.target_text,
            Self::TargetToSource => &entry.source_text,
        }
    }

    /// The language the correct answer (and every distractor) must be in.
    pub fn answer_language(self, entry: &VocabularyEntry) -> &str {
        match self {
            Self::SourceToTarget => &entry.target_language,
            Self::TargetToSource => &entry.source_language,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::categories::CategoryId;
    use crate::vocabulary_entries::VocabularyEntryId;

    fn entry() -> VocabularyEntry {
        VocabularyEntry {
            id: VocabularyEntryId(1),
            source_language: "es".to_string(),
            source_text: "manzana".to_string(),
            target_language: "en".to_string(),
            target_text: "apple".to_string(),
            category_ids: vec![CategoryId(1)],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn source_to_target_prompts_with_source_and_answers_with_target() {
        let entry = entry();

        assert_eq!(
            TranslationDirection::SourceToTarget.prompt_text(&entry),
            "manzana"
        );
        assert_eq!(
            TranslationDirection::SourceToTarget.answer_text(&entry),
            "apple"
        );
        assert_eq!(
            TranslationDirection::SourceToTarget.answer_language(&entry),
            "en"
        );
    }

    #[test]
    fn target_to_source_prompts_with_target_and_answers_with_source() {
        let entry = entry();

        assert_eq!(
            TranslationDirection::TargetToSource.prompt_text(&entry),
            "apple"
        );
        assert_eq!(
            TranslationDirection::TargetToSource.answer_text(&entry),
            "manzana"
        );
        assert_eq!(
            TranslationDirection::TargetToSource.answer_language(&entry),
            "es"
        );
    }

    #[test]
    fn round_trips_through_display_and_from_str() {
        for direction in [
            TranslationDirection::SourceToTarget,
            TranslationDirection::TargetToSource,
        ] {
            let parsed: TranslationDirection = direction.to_string().parse().unwrap();
            assert_eq!(parsed, direction);
        }
    }

    #[test]
    fn rejects_an_unrecognized_direction() {
        assert!("sideways".parse::<TranslationDirection>().is_err());
    }
}
