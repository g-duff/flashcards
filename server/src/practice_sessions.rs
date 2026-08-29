//! Practice Session domain: a bounded activity for one Learner, one
//! Category, and one Translation Direction, whose questions are generated
//! once and immutably snapshotted (grilled-spec.md sec. 2; ticket 07).

pub mod repository;

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, TimeDelta, Utc};
use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::categories::CategoryId;
use crate::learners::LearnerId;
use crate::translation_direction::TranslationDirection;
use crate::vocabulary_entries::{VocabularyEntry, VocabularyEntryId};

/// A Practice Session's durable identity. Newtype over the raw row ID
/// (server/CODING_STANDARDS.md sec. 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct PracticeSessionId(pub i64);

impl fmt::Display for PracticeSessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// A generated Question's durable identity. Newtype over the raw row ID
/// (server/CODING_STANDARDS.md sec. 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct PracticeQuestionId(pub i64);

impl fmt::Display for PracticeQuestionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Completed,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("\"{0}\" is not a recognized session status")]
pub struct ParseSessionStatusError(String);

impl fmt::Display for SessionStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let raw = match self {
            Self::Active => "active",
            Self::Completed => "completed",
        };
        write!(formatter, "{raw}")
    }
}

impl FromStr for SessionStatus {
    type Err = ParseSessionStatusError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw {
            "active" => Ok(Self::Active),
            "completed" => Ok(Self::Completed),
            other => Err(ParseSessionStatusError(other.to_string())),
        }
    }
}

/// A validated requested question count, within the server-configured
/// bounds (spec.md story 34; ticket 07).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionCount(pub u32);

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum QuestionCountError {
    #[error("question count must be between {min} and {max}")]
    OutOfRange { min: u32, max: u32 },
}

/// Validates a requested question count against the server-configured
/// min/max bounds (spec.md story 34; ticket 07).
pub fn validate_question_count(
    requested: u32,
    min: u32,
    max: u32,
) -> Result<QuestionCount, QuestionCountError> {
    if requested < min || requested > max {
        return Err(QuestionCountError::OutOfRange { min, max });
    }
    Ok(QuestionCount(requested))
}

/// One multiple-choice option in a generated Question's immutable
/// snapshot, including the `is_correct`/`is_dont_know` flags that must
/// never reach the public API before an Answer Submission exists (ticket
/// 07, 08).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionSnapshot {
    pub id: u32,
    pub text: String,
    pub is_correct: bool,
    pub is_dont_know: bool,
}

/// The public, pre-submission view of an option: `is_correct` is withheld
/// (grilled-spec.md sec. 5).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PublicOption {
    pub id: u32,
    pub text: String,
    pub is_dont_know: bool,
}

impl OptionSnapshot {
    pub fn to_public(&self) -> PublicOption {
        PublicOption {
            id: self.id,
            text: self.text.clone(),
            is_dont_know: self.is_dont_know,
        }
    }
}

/// One immutably snapshotted Question, generated once at session-creation
/// time (grilled-spec.md sec. 2).
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedQuestion {
    pub vocabulary_entry_id: VocabularyEntryId,
    pub direction: TranslationDirection,
    pub prompt_text: String,
    pub options: Vec<OptionSnapshot>,
}

impl GeneratedQuestion {
    pub fn correct_text(&self) -> &str {
        self.options
            .iter()
            .find(|option| option.is_correct)
            .map(|option| option.text.as_str())
            .unwrap_or_default()
    }
}

/// A Practice Session's Question as returned by the public API: the
/// immutable snapshot with `is_correct` withheld (ticket 07).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PracticeQuestion {
    pub id: PracticeQuestionId,
    pub vocabulary_entry_id: VocabularyEntryId,
    pub direction: TranslationDirection,
    pub ordinal: u32,
    pub prompt_text: String,
    pub options: Vec<PublicOption>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PracticeSession {
    pub id: PracticeSessionId,
    pub learner_id: LearnerId,
    pub category_id: CategoryId,
    pub direction: TranslationDirection,
    pub status: SessionStatus,
    pub requested_question_count: u32,
    pub actual_question_count: u32,
    pub answered_question_count: u32,
    pub correct_count: u32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_activity_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub questions: Vec<PracticeQuestion>,
}

/// Everything session generation needs, gathered by the imperative shell
/// so the selection/eligibility/shuffling logic below stays pure (server/
/// CODING_STANDARDS.md sec. 6).
pub struct GenerateSessionInput<'a> {
    pub category_id: CategoryId,
    pub direction: TranslationDirection,
    pub requested_count: u32,
    /// Vocabulary Entries belonging to the selected Category: the
    /// candidate pool for questions.
    pub category_entries: &'a [VocabularyEntry],
    /// Every Vocabulary Entry, used to source distractors from the same
    /// Language Pair when the selected Category can't supply enough
    /// (grilled-spec.md sec. 4).
    pub all_entries: &'a [VocabularyEntry],
    pub last_correct_at: &'a HashMap<(VocabularyEntryId, TranslationDirection), DateTime<Utc>>,
    pub min_interval_before_retest_days: f64,
    /// The number of distinct incorrect distractors each Question must
    /// have (server-configured; `config.yaml`'s `incorrect_distractor_count`).
    pub incorrect_distractor_count: usize,
    pub now: DateTime<Utc>,
}

/// Generates the Question set for a new Practice Session: filters
/// candidate entries down to Eligible Entries (Category membership,
/// retest cooldown, distractor availability), selects up to
/// `requested_count` of them, and shuffles both the selected entries'
/// order and each Question's option order (grilled-spec.md sec. 4; ticket
/// 07).
///
/// Entry selection is not yet priority-ranked — every Eligible Entry is
/// currently weighted equally, so candidates are taken in the stable order
/// they're passed in (`category_entries`, which repository callers sort by
/// creation date). Ticket 10 replaces this with the full priority formula.
pub fn generate_session_questions(
    input: GenerateSessionInput<'_>,
    rng: &mut impl Rng,
) -> Vec<GeneratedQuestion> {
    let eligible: Vec<GeneratedQuestion> = input
        .category_entries
        .iter()
        .filter(|entry| {
            !is_in_cooldown(
                input
                    .last_correct_at
                    .get(&(entry.id, input.direction))
                    .copied(),
                input.min_interval_before_retest_days,
                input.now,
            )
        })
        .filter_map(|entry| {
            let distractors = find_distractor_texts(
                entry,
                input.direction,
                input.category_id,
                input.all_entries,
                input.incorrect_distractor_count,
            )?;
            Some(build_question(entry, input.direction, distractors, rng))
        })
        .collect();

    let selected_count = (input.requested_count as usize).min(eligible.len());
    let mut selected: Vec<GeneratedQuestion> = eligible.into_iter().take(selected_count).collect();
    selected.shuffle(rng);
    selected
}

/// Whether `last_correct_at` falls within the hard retest cooldown as of
/// `now` (grilled-spec.md sec. 4, eligibility rule 2). No prior correct
/// answer means the entry is never in cooldown.
fn is_in_cooldown(
    last_correct_at: Option<DateTime<Utc>>,
    min_interval_before_retest_days: f64,
    now: DateTime<Utc>,
) -> bool {
    let Some(last_correct_at) = last_correct_at else {
        return false;
    };
    let min_interval =
        TimeDelta::milliseconds((min_interval_before_retest_days * 86_400_000.0).round() as i64);
    now - last_correct_at < min_interval
}

/// Finds up to `incorrect_distractor_count` distinct distractor texts in
/// `entry`'s answer language, sourced from other Vocabulary Entries
/// sharing its Language Pair. The selected Category is preferred; other
/// Categories in the same Language Pair fill any remaining slots
/// (grilled-spec.md sec. 4). Returns `None` when fewer than
/// `incorrect_distractor_count` distinct incorrect options can be found,
/// meaning `entry` is not Eligible.
fn find_distractor_texts(
    entry: &VocabularyEntry,
    direction: TranslationDirection,
    selected_category_id: CategoryId,
    all_entries: &[VocabularyEntry],
    incorrect_distractor_count: usize,
) -> Option<Vec<String>> {
    let answer_language = direction.answer_language(entry);
    let correct_text = direction.answer_text(entry);
    let pair = language_pair(entry);

    let mut same_category = Vec::new();
    let mut other_category = Vec::new();

    for other in all_entries {
        if other.id == entry.id || language_pair(other) != pair {
            continue;
        }
        let Some(text) = text_in_language(other, answer_language) else {
            continue;
        };
        if text == correct_text {
            continue;
        }

        if other.category_ids.contains(&selected_category_id) {
            same_category.push(text);
        } else {
            other_category.push(text);
        }
    }

    let mut distinct = Vec::with_capacity(incorrect_distractor_count);
    for text in same_category.into_iter().chain(other_category) {
        if distinct.len() == incorrect_distractor_count {
            break;
        }
        if !distinct.contains(&text) {
            distinct.push(text);
        }
    }

    (distinct.len() == incorrect_distractor_count).then_some(distinct)
}

/// The unordered Language Pair an entry belongs to (grilled-spec.md sec.
/// 2).
fn language_pair(entry: &VocabularyEntry) -> (String, String) {
    let mut languages = [entry.source_language.clone(), entry.target_language.clone()];
    languages.sort();
    let [first, second] = languages;
    (first, second)
}

/// The text `entry` holds in `language`, whichever side that is, if any.
fn text_in_language(entry: &VocabularyEntry, language: &str) -> Option<String> {
    if entry.target_language == language {
        Some(entry.target_text.clone())
    } else if entry.source_language == language {
        Some(entry.source_text.clone())
    } else {
        None
    }
}

/// Builds one Question's immutable option snapshot: the correct answer
/// plus its incorrect distractors, shuffled together, with `Don't know`
/// always last (grilled-spec.md sec. 3, 5).
fn build_question(
    entry: &VocabularyEntry,
    direction: TranslationDirection,
    distractor_texts: Vec<String>,
    rng: &mut impl Rng,
) -> GeneratedQuestion {
    let mut translation_options: Vec<OptionSnapshot> = distractor_texts
        .into_iter()
        .map(|text| OptionSnapshot {
            id: 0,
            text,
            is_correct: false,
            is_dont_know: false,
        })
        .collect();
    translation_options.push(OptionSnapshot {
        id: 0,
        text: direction.answer_text(entry).to_string(),
        is_correct: true,
        is_dont_know: false,
    });
    translation_options.shuffle(rng);

    let mut options: Vec<OptionSnapshot> = translation_options
        .into_iter()
        .enumerate()
        .map(|(index, option)| OptionSnapshot {
            id: (index + 1) as u32,
            ..option
        })
        .collect();
    options.push(OptionSnapshot {
        id: (options.len() + 1) as u32,
        text: "Don't know".to_string(),
        is_correct: false,
        is_dont_know: true,
    });

    GeneratedQuestion {
        vocabulary_entry_id: entry.id,
        direction,
        prompt_text: direction.prompt_text(entry).to_string(),
        options,
    }
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use super::*;

    fn entry(
        id: i64,
        source_language: &str,
        source_text: &str,
        target_language: &str,
        target_text: &str,
        category_ids: Vec<CategoryId>,
    ) -> VocabularyEntry {
        VocabularyEntry {
            id: VocabularyEntryId(id),
            source_language: source_language.to_string(),
            source_text: source_text.to_string(),
            target_language: target_language.to_string(),
            target_text: target_text.to_string(),
            category_ids,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    #[test]
    fn accepts_a_count_within_bounds() {
        assert_eq!(validate_question_count(15, 10, 20), Ok(QuestionCount(15)));
    }

    #[test]
    fn rejects_a_count_below_the_minimum() {
        assert_eq!(
            validate_question_count(5, 10, 20),
            Err(QuestionCountError::OutOfRange { min: 10, max: 20 })
        );
    }

    #[test]
    fn rejects_a_count_above_the_maximum() {
        assert_eq!(
            validate_question_count(25, 10, 20),
            Err(QuestionCountError::OutOfRange { min: 10, max: 20 })
        );
    }

    #[test]
    fn no_prior_correct_answer_is_never_in_cooldown() {
        assert!(!is_in_cooldown(None, 1.0, Utc::now()));
    }

    #[test]
    fn recent_correct_answer_is_in_cooldown() {
        let now = Utc::now();
        let last_correct_at = now - TimeDelta::hours(1);

        assert!(is_in_cooldown(Some(last_correct_at), 1.0, now));
    }

    #[test]
    fn correct_answer_outside_the_interval_is_not_in_cooldown() {
        let now = Utc::now();
        let last_correct_at = now - TimeDelta::days(2);

        assert!(!is_in_cooldown(Some(last_correct_at), 1.0, now));
    }

    fn fruit() -> CategoryId {
        CategoryId(1)
    }
    fn snacks() -> CategoryId {
        CategoryId(2)
    }

    // Distractors are sourced from the selected Category first (ticket 07).
    #[test]
    fn finds_distractors_from_the_selected_category_first() {
        let target = entry(1, "es", "manzana", "en", "apple", vec![fruit()]);
        let others = vec![
            target.clone(),
            entry(2, "es", "naranja", "en", "orange", vec![fruit()]),
            entry(3, "es", "platano", "en", "banana", vec![fruit()]),
            entry(4, "es", "uva", "en", "grape", vec![fruit()]),
            entry(5, "es", "pera", "en", "pear", vec![fruit()]),
            // Same language pair, different category: should not be needed.
            entry(6, "es", "hermano", "en", "brother", vec![snacks()]),
        ];

        let distractors = find_distractor_texts(
            &target,
            TranslationDirection::SourceToTarget,
            fruit(),
            &others,
            4,
        )
        .expect("expected four distractors");

        assert_eq!(distractors.len(), 4);
        assert!(!distractors.contains(&"brother".to_string()));
    }

    // When the selected Category can't supply four distinct distractors,
    // the server falls back to other Categories in the same Language Pair
    // (grilled-spec.md sec. 4; ticket 07).
    #[test]
    fn falls_back_to_other_categories_in_the_same_language_pair() {
        let target = entry(1, "es", "manzana", "en", "apple", vec![fruit()]);
        let others = vec![
            target.clone(),
            entry(2, "es", "naranja", "en", "orange", vec![fruit()]),
            entry(3, "es", "hermano", "en", "brother", vec![snacks()]),
            entry(4, "es", "hermana", "en", "sister", vec![snacks()]),
            entry(5, "es", "perro", "en", "dog", vec![snacks()]),
        ];

        let distractors = find_distractor_texts(
            &target,
            TranslationDirection::SourceToTarget,
            fruit(),
            &others,
            4,
        )
        .expect("expected four distractors via fallback");

        assert_eq!(distractors.len(), 4);
        assert!(distractors.contains(&"brother".to_string()));
    }

    // An entry whose Language Pair can't produce four distinct incorrect
    // options is omitted (not Eligible) (grilled-spec.md sec. 4; ticket 07).
    #[test]
    fn returns_none_when_fewer_than_four_distractors_exist() {
        let target = entry(1, "es", "manzana", "en", "apple", vec![fruit()]);
        let others = vec![
            target.clone(),
            entry(2, "es", "naranja", "en", "orange", vec![fruit()]),
        ];

        assert_eq!(
            find_distractor_texts(
                &target,
                TranslationDirection::SourceToTarget,
                fruit(),
                &others,
                4,
            ),
            None
        );
    }

    // Distractor texts must be distinct: a duplicate answer-language text
    // across two other entries only counts once (grilled-spec.md sec. 4).
    #[test]
    fn deduplicates_distractor_texts() {
        let target = entry(1, "es", "manzana", "en", "apple", vec![fruit()]);
        let others = vec![
            target.clone(),
            entry(2, "es", "naranja", "en", "orange", vec![fruit()]),
            entry(3, "fr", "orange", "en", "orange", vec![fruit()]),
            entry(4, "es", "platano", "en", "banana", vec![fruit()]),
            entry(5, "es", "uva", "en", "grape", vec![fruit()]),
            entry(6, "es", "pera", "en", "pear", vec![fruit()]),
        ];

        let distractors = find_distractor_texts(
            &target,
            TranslationDirection::SourceToTarget,
            fruit(),
            &others,
            4,
        )
        .expect("expected four distractors");

        let orange_count = distractors.iter().filter(|text| *text == "orange").count();
        assert_eq!(orange_count, 1);
    }

    fn eligible_pool() -> Vec<VocabularyEntry> {
        vec![
            entry(1, "es", "manzana", "en", "apple", vec![fruit()]),
            entry(2, "es", "naranja", "en", "orange", vec![fruit()]),
            entry(3, "es", "platano", "en", "banana", vec![fruit()]),
            entry(4, "es", "uva", "en", "grape", vec![fruit()]),
            entry(5, "es", "pera", "en", "pear", vec![fruit()]),
        ]
    }

    // A Category with enough distinct entries generates a full session
    // (ticket 07).
    #[test]
    fn generates_the_requested_number_of_questions_when_enough_are_eligible() {
        let pool = eligible_pool();
        let questions = generate_session_questions(
            GenerateSessionInput {
                category_id: fruit(),
                direction: TranslationDirection::SourceToTarget,
                requested_count: 3,
                category_entries: &pool,
                all_entries: &pool,
                last_correct_at: &HashMap::new(),
                min_interval_before_retest_days: 1.0,
                incorrect_distractor_count: 4,
                now: Utc::now(),
            },
            &mut rng(),
        );

        assert_eq!(questions.len(), 3);
    }

    // Fewer eligible entries than requested still produces a session, with
    // the available count (grilled-spec.md sec. 4; ticket 07).
    #[test]
    fn generates_fewer_questions_than_requested_when_that_is_all_that_is_available() {
        let pool = eligible_pool();
        let questions = generate_session_questions(
            GenerateSessionInput {
                category_id: fruit(),
                direction: TranslationDirection::SourceToTarget,
                requested_count: 20,
                category_entries: &pool,
                all_entries: &pool,
                last_correct_at: &HashMap::new(),
                min_interval_before_retest_days: 1.0,
                incorrect_distractor_count: 4,
                now: Utc::now(),
            },
            &mut rng(),
        );

        assert_eq!(questions.len(), 5);
    }

    // Zero eligible entries produces zero questions rather than an error;
    // the HTTP layer turns this into a clear, non-500 response (grilled-
    // spec.md sec. 9; ticket 07).
    #[test]
    fn generates_no_questions_when_none_are_eligible() {
        let pool = vec![entry(1, "es", "manzana", "en", "apple", vec![fruit()])];
        let questions = generate_session_questions(
            GenerateSessionInput {
                category_id: fruit(),
                direction: TranslationDirection::SourceToTarget,
                requested_count: 5,
                category_entries: &pool,
                all_entries: &pool,
                last_correct_at: &HashMap::new(),
                min_interval_before_retest_days: 1.0,
                incorrect_distractor_count: 4,
                now: Utc::now(),
            },
            &mut rng(),
        );

        assert!(questions.is_empty());
    }

    // An entry within its hard retest cooldown is excluded (grilled-spec.md
    // sec. 4, eligibility rule 2; ticket 07).
    #[test]
    fn excludes_entries_within_the_retest_cooldown() {
        let pool = eligible_pool();
        let mut last_correct_at = HashMap::new();
        last_correct_at.insert(
            (VocabularyEntryId(1), TranslationDirection::SourceToTarget),
            Utc::now(),
        );

        let questions = generate_session_questions(
            GenerateSessionInput {
                category_id: fruit(),
                direction: TranslationDirection::SourceToTarget,
                requested_count: 5,
                category_entries: &pool,
                all_entries: &pool,
                last_correct_at: &last_correct_at,
                min_interval_before_retest_days: 1.0,
                incorrect_distractor_count: 4,
                now: Utc::now(),
            },
            &mut rng(),
        );

        assert_eq!(questions.len(), 4);
        assert!(
            questions
                .iter()
                .all(|question| question.vocabulary_entry_id != VocabularyEntryId(1))
        );
    }

    // Every generated Question has exactly one correct option, four
    // incorrect translation options, and a trailing "Don't know" option
    // (grilled-spec.md sec. 3, 5; ticket 07).
    #[test]
    fn each_question_has_one_correct_option_and_a_trailing_dont_know() {
        let pool = eligible_pool();
        let questions = generate_session_questions(
            GenerateSessionInput {
                category_id: fruit(),
                direction: TranslationDirection::SourceToTarget,
                requested_count: 1,
                category_entries: &pool,
                all_entries: &pool,
                last_correct_at: &HashMap::new(),
                min_interval_before_retest_days: 1.0,
                incorrect_distractor_count: 4,
                now: Utc::now(),
            },
            &mut rng(),
        );

        let question = &questions[0];
        assert_eq!(question.options.len(), 6);
        assert_eq!(question.options.iter().filter(|o| o.is_correct).count(), 1);
        assert_eq!(
            question
                .options
                .iter()
                .filter(|o| !o.is_correct && !o.is_dont_know)
                .count(),
            4
        );
        let last = question.options.last().unwrap();
        assert!(last.is_dont_know);
        assert_eq!(last.text, "Don't know");
    }
}
