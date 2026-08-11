# Language Practice Flashcards

The shared learning domain for practicing translation pairs across named local learner profiles.

## Core Concepts

**Vocabulary Entry**:
A translation pair containing source and target text plus their languages; it may be a word or a short phrase.
_Avoid_: Vocab Word, Word

**Translation Direction**:
The direction in which a Vocabulary Entry is practiced, from one language side to the other.
_Avoid_: Mode

**Learner**:
A durable local profile whose practice progress is tracked independently.
_Avoid_: Account, Authenticated User

**Shared Content**:
Vocabulary Entries and Categories available to every Learner in the app.
_Avoid_: User-owned Content, Private Content

**Category Membership**:
The association between a Vocabulary Entry and one or more Categories; every Vocabulary Entry must have at least one membership.
_Avoid_: Category ownership

## Practice

**Practice Session**:
A bounded learning activity containing an immutable set of generated questions for one Learner and Category; it is either active or completed.
_Avoid_: Quiz, Attempt

**Answer Submission**:
The explicit answer action that scores a displayed question; temporary UI selection is not an Answer Submission.
_Avoid_: Click, Attempt

**Don't Know Response**:
An explicit incorrect Answer Submission indicating that the Learner does not know the answer.
_Avoid_: Skip, Unanswered

**Discarded Session**:
An unfinished Practice Session that is removed rather than retained as historical activity.
_Avoid_: Abandoned Session

**Direction-specific Progress**:
A Learner's knowledge state for one Vocabulary Entry in one Translation Direction.
_Avoid_: Word Progress, Overall Progress

**Learner Algorithm Settings**:
The complete set of spaced-repetition tuning parameters used for one Learner, initialized from application defaults and applied to that Learner's future sessions.
_Avoid_: Global Settings, Preset

**Eligible Entry**:
A Vocabulary Entry and Translation Direction that can be selected because it has sufficient distractors and is outside its retest cooldown.
_Avoid_: Available Word

**Vocabulary Deletion**:
Removal of a Vocabulary Entry and its associated Category Memberships, Direction-specific Progress, and practice history.
_Avoid_: Archive, Soft Delete

**Editable Shared Content**:
Shared Categories and Vocabulary Entries may be corrected or reorganized after creation without recreating them.
_Avoid_: Immutable Content

**Language Pair**:
The unordered pair of languages represented by a Vocabulary Entry; practice direction is modeled separately.
_Avoid_: Directional Pair
