# Grilled Language-Practice Flashcards Specification

This document supersedes the decisions in `01-enhanced-spec.md` where they conflict. It records the design agreed during the grilling session.

## 1. Scope and Technology

### In scope

- Durable local Learners
- Shared Categories and Vocabulary Entries
- Multiple-choice Practice Sessions
- Direction-specific Progress
- Configurable spaced repetition
- Category proficiency and aggregate statistics
- Client-side CSV parsing with atomic bulk import
- SQLite persistence
- Docker Compose for separate client and server services

### Deferred

- Text-based typed practice
- Offline localStorage caching and synchronization
- Production authentication or authorization
- Multiple target translations per Vocabulary Entry
- Cross-device synchronization
- Advanced analytics, polish, and performance optimization
- Supporting multiple database engines in the first implementation
- Server-side CSV parsing

### Stack

- Client: TypeScript, React, web components, optimized for iPhone Safari
- Server: Rust and axum
- Persistence: SQLite accessed with raw SQL
- Configuration: YAML file with configurable database URL and migration path
- Tests: Vitest for client behavior and HTTP integration tests against the axum API with an isolated SQLite database
- Build/lint/format: existing npm scripts and repository conventions

## 2. Domain Model

### Learner

A durable local profile with a unique display name. Progress is isolated per Learner. Names are compared case-insensitively after trimming surrounding whitespace, while display casing is preserved. Learners may be renamed or explicitly deleted.

The current-learner cookie stores the durable Learner ID, not the name. It is host-only, `HttpOnly`, `SameSite=Lax`, and has an explicit long expiry. An invalid cookie is cleared and the client redirects to Home.

### Vocabulary Entry

A language-neutral translation pair containing arbitrary short source and target text plus two ISO 639-1 language codes. It may represent a word or phrase, including an article such as `la manzana`.

One Vocabulary Entry has exactly one target text. Alternate translations are separate Vocabulary Entries. Duplicate language-and-text pairs are rejected globally. Surrounding whitespace and Unicode form are normalized for identity checks; case, punctuation, and accents remain meaningful.

Source and target languages are immutable after creation. Text and Category Memberships may be edited without changing the Vocabulary Entry identity or its existing progress.

### Language Pair and Translation Direction

A Language Pair is the unordered pair of languages in a Vocabulary Entry. A Translation Direction chooses one side as the prompt and the other as the answer.

Each Practice Session uses exactly one Translation Direction. Progress is tracked independently for each Learner, Vocabulary Entry, and Translation Direction.

### Category and Category Membership

Categories are shared content available to every Learner. A Vocabulary Entry must have at least one Category Membership and may belong to many Categories.

Category names are compared case-insensitively after trimming whitespace. Deleting a Category is rejected if it would remove the final Category Membership from any Vocabulary Entry. Category deletion does not cascade-delete Vocabulary Entries.

### Practice Session

A bounded activity for one Learner, one Category, and one Translation Direction. A session is either `active` or `completed`.

At creation, the server snapshots every question's prompt text, correct answer text, direction, distractor texts, option identities, and ordering. Later edits or deletions cannot rewrite the snapshot.

Only one active session may exist for a Learner globally. Restarting explicitly discards the active session and creates a new one. Inactivity timeout also discards it. Discarded sessions are removed and do not contribute to statistics.

### Question and Answer Submission

Each question displays exactly four translation options plus `Don't know`. There is exactly one correct option. The correct multiple-choice response must exactly match the stored target text for the selected direction.

Temporary UI selection is not scored. A question accepts one explicit Answer Submission. Duplicate submissions are idempotent and cannot update progress twice. `Don't know` is an explicit incorrect Answer Submission.

### Direction-specific Progress

Progress is created lazily after the first Answer Submission for a Learner/Vocabulary Entry/Translation Direction combination.

Tracked values:

- `total_correct_count`
- `total_incorrect_count`
- `current_correct_streak`
- `last_correct_at`
- `last_incorrect_at`
- `created_at`
- `updated_at`

Correct answers increment the total and streak. Incorrect answers, including `Don't know`, increment the incorrect total, reset the streak, and update the incorrect timestamp.

### Learner Algorithm Settings

Each Learner stores a complete copy of the spaced-repetition tuning parameters. New Learners receive values from YAML application defaults. Existing stored settings take precedence over later file-default changes. Learners may edit or reset their settings through a dedicated settings surface.

## 3. User Interface

### Home

- Create or select a Learner
- Display the current Learner
- Navigate to Categories, Add Vocabulary, Progress, and Settings
- Clear invalid identity state by returning to Home

### Choose Practice Category

- List Categories by creation date or alphabetically
- Show overall and separate-direction proficiency for the current Learner
- Select a Category, Translation Direction, and requested question count
- Start or restart a Practice Session

### Practice

- Show progress such as `5/20`
- Show one prompt and five responses: four translations plus `Don't know`
- Allow temporary selection before explicit submission
- Show immediate feedback after submission
- Advance only after the answer has been submitted
- Show a summary after the final answer

### Add Vocabulary

- Add one or more Vocabulary Entries
- Collect source text, target text, languages, and one or more Categories
- Paste CSV using `source | target | category`
- Resolve Categories by normalized name
- Show validation or duplicate errors
- Commit the complete bulk import atomically

### Learner Settings

- Show current Learner Algorithm Settings
- Validate and save per-Learner tuning values
- Reset settings to current application defaults

## 4. Spaced Repetition

### Default application settings

```yaml
correct_streak_threshold: 5
incorrect_boost_threshold: 2
deprioritize_duration_days: 3
prioritize_duration_days: 2
min_interval_before_retest_days: 1
incorrect_weight: 3.0
time_decay_factor: 0.5
base_priority: 0.0
question_count_min: 10
question_count_max: 20
session_inactivity_timeout_minutes: 30
```

The threshold and duration settings are persisted even where a particular scoring rule uses them differently. Configuration values are validated before saving.

### Eligibility

A Vocabulary Entry and Translation Direction is Eligible only when:

1. It belongs to the selected Category.
2. It is outside the hard retest cooldown after its latest correct answer.
3. The selected Language Pair has enough distinct entries to generate four incorrect distractors.

The selected Category is preferred for distractors. If it cannot provide enough distinct options, the server fills from other Categories in the same Language Pair. If four distinct incorrect options still cannot be generated, the entry is omitted from that session.

### Priority

For each Eligible Entry and direction:

```text
priority = base_priority

if elapsed_since_last_correct > min_interval_before_retest:
    priority += elapsed_since_last_correct_days * time_decay_factor

priority += total_incorrect_count * incorrect_weight

if current_correct_streak >= correct_streak_threshold:
    priority -= deprioritize_duration_days * 10
```

The server selects the highest-priority entries, then shuffles question order. If fewer valid questions are available than requested, the session contains the available count. If none are available, no session is created and the client explains why.

All elapsed-time calculations use exact UTC durations.

## 5. API Contracts

### Identity and envelope

Learner-scoped operations derive identity from the current-learner cookie. Request bodies must not allow a caller to act as another Learner by supplying an arbitrary user ID.

Success responses use:

```json
{
  "status": "success",
  "data": {},
  "meta": {
    "timestamp": "2026-08-11T17:56:16Z"
  }
}
```

Error responses use:

```json
{
  "status": "error",
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "User-friendly error message",
    "details": [
      { "field": "source_text", "reason": "Cannot be empty" }
    ]
  },
  "meta": {
    "timestamp": "2026-08-11T17:56:16Z"
  }
}
```

Optional empty fields are omitted. Successful deletion returns the success envelope without a `data: null` field.

### Learners

| Method | Route | Purpose |
|---|---|---|
| `POST` | `/api/learners` | Create a Learner and set the current-learner cookie |
| `GET` | `/api/learners` | List Learners for profile selection |
| `GET` | `/api/learners/:id` | Read a Learner |
| `PATCH` | `/api/learners/:id` | Rename a Learner |
| `DELETE` | `/api/learners/:id` | Delete a Learner and personal dependent data |
| `POST` | `/api/session/learner` | Select a Learner and set the cookie |
| `DELETE` | `/api/session/learner` | Clear the current-learner cookie |

### Categories

| Method | Route | Purpose |
|---|---|---|
| `POST` | `/api/categories` | Create a shared Category |
| `GET` | `/api/categories` | List Categories with current-Learner proficiency |
| `GET` | `/api/categories/:id` | Read a Category |
| `PATCH` | `/api/categories/:id` | Rename a Category |
| `DELETE` | `/api/categories/:id` | Delete a Category if no entry would be orphaned |

### Vocabulary Entries

| Method | Route | Purpose |
|---|---|---|
| `POST` | `/api/vocabulary-entries` | Create one Vocabulary Entry |
| `POST` | `/api/vocabulary-entries/bulk` | Atomically create multiple entries |
| `GET` | `/api/vocabulary-entries` | List entries, optionally filtered by Category or Language Pair |
| `GET` | `/api/vocabulary-entries/:id` | Read an entry |
| `PATCH` | `/api/vocabulary-entries/:id` | Edit text or Category Memberships; languages remain immutable |
| `DELETE` | `/api/vocabulary-entries/:id` | Delete entry, memberships, progress, and practice history |

Example create request:

```json
{
  "source_language": "es",
  "source_text": "manzana",
  "target_language": "en",
  "target_text": "apple",
  "category_ids": [1]
}
```

### Practice Sessions

| Method | Route | Purpose |
|---|---|---|
| `POST` | `/api/practice-sessions` | Create an active session for the current Learner |
| `GET` | `/api/practice-sessions/:id` | Read the session snapshot and status |
| `POST` | `/api/practice-sessions/:id/restart` | Discard the active session and create a replacement |
| `POST` | `/api/practice-sessions/:id/questions/:question_id/submit` | Submit one explicit answer |
| `POST` | `/api/practice-sessions/:id/complete` | Idempotent completion confirmation |

Start request:

```json
{
  "category_id": 1,
  "direction": "source_to_target",
  "question_count": 10
}
```

Question response shape:

```json
{
  "id": 1,
  "prompt_text": "manzana",
  "direction": "source_to_target",
  "options": [
    { "id": 1, "text": "apple", "is_correct": false },
    { "id": 2, "text": "orange", "is_correct": false },
    { "id": 3, "text": "apple", "is_correct": true },
    { "id": 4, "text": "brother", "is_correct": false },
    { "id": 5, "text": "Don't know", "is_dont_know": true }
  ]
}
```

The public response must not reveal `is_correct` before submission; the shape above describes the snapshot model, not an unredacted client response.

Submit request:

```json
{
  "option_id": 3
}
```

The response includes correctness, feedback, and the next unanswered question when one exists. Repeating a submission returns the existing result without applying progress again.

### Progress and statistics

| Method | Route | Purpose |
|---|---|---|
| `GET` | `/api/me/progress` | Detailed Direction-specific Progress for the current Learner |
| `GET` | `/api/me/stats` | Current Learner aggregate statistics |
| `GET` | `/api/stats` | Aggregate statistics across Learners |

Deleted Vocabulary Entries and discarded sessions are excluded from current statistics.

### Learner Algorithm Settings

| Method | Route | Purpose |
|---|---|---|
| `GET` | `/api/me/algorithm-settings` | Read persisted settings |
| `PATCH` | `/api/me/algorithm-settings` | Update validated per-Learner settings |
| `POST` | `/api/me/algorithm-settings/reset` | Copy current YAML defaults into the Learner record |

### HTTP status behavior

- `200 OK`: successful read, update, submission, completion, or deletion
- `201 Created`: successful creation
- `400 Bad Request`: validation failure
- `404 Not Found`: missing resource
- `409 Conflict`: duplicate, unsafe deletion, or active-session conflict
- `500 Internal Server Error`: unexpected server failure

## 6. Persistence Model

```text
learners
  1 ──────────────── M learner_algorithm_settings
  1 ──────────────── M direction_progress
  1 ──────────────── M practice_sessions

categories
  1 ──────────────── M category_memberships M ──────────────── 1 vocabulary_entries

vocabulary_entries
  1 ──────────────── M direction_progress
  1 ──────────────── M practice_questions

practice_sessions
  1 ──────────────── M practice_questions
  1 ──────────────── M practice_answer_submissions

practice_questions
  1 ──────────────── 0..1 practice_answer_submissions
```

### Tables

```text
learners
- id PK
- name
- normalized_name UNIQUE
- created_at
- updated_at
```

```text
learner_algorithm_settings
- learner_id PK/FK learners
- correct_streak_threshold
- incorrect_boost_threshold
- deprioritize_duration_days
- prioritize_duration_days
- min_interval_before_retest_days
- incorrect_weight
- time_decay_factor
- base_priority
- updated_at
```

```text
categories
- id PK
- name
- normalized_name UNIQUE
- created_at
- updated_at
```

```text
vocabulary_entries
- id PK
- source_language
- source_text
- target_language
- target_text
- normalized_identity UNIQUE
- created_at
- updated_at
```

```text
category_memberships
- category_id PK/FK categories
- vocabulary_entry_id PK/FK vocabulary_entries
- created_at
```

```text
direction_progress
- id PK
- learner_id FK learners
- vocabulary_entry_id FK vocabulary_entries
- direction
- total_correct_count
- total_incorrect_count
- current_correct_streak
- last_correct_at
- last_incorrect_at
- created_at
- updated_at
- UNIQUE(learner_id, vocabulary_entry_id, direction)
```

```text
practice_sessions
- id PK
- learner_id FK learners
- category_id FK categories
- direction
- status
- requested_question_count
- actual_question_count
- answered_question_count
- correct_count
- started_at
- completed_at
- last_activity_at
- created_at
- updated_at
```

```text
practice_questions
- id PK
- session_id FK practice_sessions
- vocabulary_entry_id FK vocabulary_entries
- direction
- ordinal
- prompt_text_snapshot
- correct_text_snapshot
- options_snapshot JSON
- created_at
```

```text
practice_answer_submissions
- id PK
- question_id UNIQUE/FK practice_questions
- selected_option_snapshot JSON
- is_correct
- is_dont_know
- answered_at
```

Deleting a Learner removes that Learner's settings, progress, sessions, questions, and submissions, but never shared Categories or Vocabulary Entries. Deleting a Vocabulary Entry removes its memberships, direction progress, questions, submissions, and any associated practice-history contribution.

## 7. User-Story Flows

### Create shared content

1. Create or select a Learner.
2. Create a Category.
3. Create a Vocabulary Entry with source/target text, language codes, and at least one Category.
4. Repeat or use atomic bulk import.

### Practice

1. Select a Category and Translation Direction.
2. Request a valid question count.
3. The server creates an immutable session snapshot.
4. Select an option temporarily.
5. Explicitly submit the selected option or `Don't know`.
6. The server records the answer and updates direction-specific progress transactionally.
7. Repeat until every question is answered.
8. The server marks the session completed and returns the summary.

### Restart

1. The Learner requests restart for the active session.
2. The server discards the old session.
3. The server generates a replacement session under the one-active-session rule.

## 8. Testing Decisions

The primary seam is black-box HTTP integration testing against axum with an isolated SQLite database. Tests should assert external behavior: HTTP status, cookies, redirects, response envelopes, omitted fields, persisted outcomes visible through subsequent API calls, and transactional atomicity.

Cover:

- Learner creation, selection, renaming, deletion, cookie invalidation
- Category and Vocabulary Entry invariants and editing
- Atomic bulk import
- Category-first and Language-Pair distractor fallback
- Four options plus `Don't know`
- Immutable snapshots
- One-time idempotent submissions
- Automatic completion, restart, and timeout discard
- Direction-specific progress and streak behavior
- Cooldown, priority scoring, and per-Learner settings
- Detailed versus aggregate statistics
- Deletion cleanup

Focused deterministic priority tests may supplement the HTTP suite for exact elapsed-time and threshold boundaries.

## 9. Error and Edge-Case Rules

- Empty required text or invalid IDs: `400`.
- Unknown Category during bulk import: reject the entire import.
- Duplicate Learner, Category, or Vocabulary Entry identity: `409`.
- Vocabulary Entry without a Category: `400`.
- Category deletion that would orphan an entry: `409`.
- Starting a second active session without restart: `409`.
- Zero eligible questions: no session; explain that no Eligible Entries are available.
- Fewer valid questions than requested: create the session with the actual count.
- Submission for an unknown or already-invalid question: `404` or `409` as appropriate.
- Completion before every question is answered: reject; completion is automatic after the final submission.
- Invalid current-learner cookie: clear and redirect to Home.
- YAML default changes affect new Learners only until an existing Learner resets settings.
- Empty optional response properties are omitted rather than sent as `null`.

## 10. Deployment Notes

Provide separate client and server Docker Compose services. Mount a persistent volume for the SQLite database. Configure the server with YAML values for the database URL, migration path, supported language list, session bounds, timeout, cookie expiry, and initial algorithm defaults. Number and version migrations.
