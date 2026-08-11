# Enhanced Language-Practice Flashcards App Specification

## 1. Tech Spec and Repo Structure
I want a design for a flashcards app for language learning, inspired by DuoLingo.

When I am asking for a design, you must include:
* A database schema diagram in plaintext
* A list of user-stories
* A list of REST api routes
* Examples for how the user-stories are achieved using the proposed APIs

### Tech Stack
**Front-end** in `client` in typescript with web components in react. The client will primarily be used on an iPhone in the Safari browser.
* Dev dependencies accessed with these package scripts:
  * tested with vitest: `npm run test`
  * bundled with esbuild: `npm run build`
  * linted with eslint: `npm run lint`
  * formatted with prettier's default config: `npm run format`

**Back-end** in `server` directory: rust
* Web server: axum
* Database library requirements:
  * URL must be configurable from a config file
  * Config file must be yaml or json5
  * Path to migrations must be configurable from a config file
  * Preferably using raw SQL but diesel is acceptable
* Following the coding standards defined in "server/CODING_STANDARDS.md"

### Repo Structure
* `client/`: Front-end concerns
* `server/`: Backend concerns

---

## 2. Feature Priorities

### Highest: Multiple Choice Practice
The app will show one word, and multiple possible answers, one of which is correct, then the user must select the correct translations.
* Language practice may go in both directions: english→spanish and spanish→english
* Examples:
  * The app shows "hola", with 5 possible answers ["hello", "apple", "brother", "eat", "day"]. User must select "hello" to succeed.
  * The app shows "hello", with 5 possible answers ["hola", "manzana", "hermano", "como", "bienvenido"]. User must select "hola" to succeed.

### High: Vocab Categories
* Users will select a category of vocab to practice in a session
* Vocab must belong to at least one category
* Categories are user-defined and contain tens or hundreds of words
* One word may belong to multiple categories

### Medium: Progress Tracking & Spaced Repetition
* Progress tracking must support multiple users (app distinguishes between "George" and "Sam", tracking separately)
* Progress stats are viewable by all users
* App remembers the current user via cookie (login/persistence handled by cookie)
* The app prioritizes words for practice based on:
  * Frequency of incorrect answers (priority boost)
  * Time since last correct practice (priority boost after cooldown period)
  * Correct-answer streak (priority reduction after threshold)
* Algorithm is tunable via config file and/or UI
* Examples:
  * User correctly translates "la manzana" 5 times in 7 days → deprioritize for 3 days
  * User incorrectly translates "naranja" 2 times in 1 day → prioritize for 2 days
* See section 4 for detailed algorithm specification

### Low (Deferrable): Text-Based Practice
* The app shows a word, and the user types the correct answer in a text box
* Practice translations go in either direction: english→spanish and spanish→english
* Correct translation must include the correct article (e.g., "el", "la")
* Examples:
  * App shows "apple", user types "la manzana" → correct
  * App shows "apple", user types "el manzana" → incorrect (wrong article)
  * App shows "apple", user types "manzana" → incorrect (article missing)

---

## 3. UI Description

### Screens

**Home Screen**
* Allows navigation to other screens below
* Requires user to enter their name or select from existing users
* Displays current user name at top

**Choose Practice Category**
* Lists all categories (sorted by creation date or alphabetically)
* Shows proficiency indicator per category for current user (e.g., % correct, difficulty level, last practiced date)
* Click category → navigates to Practice screen
* Redirects to Home if user is not set in cookie

**Practice Screen**
* Displays practice question(s)
* Shows progress bar (e.g., "5/20 questions complete")
* Displays current question with 4-5 multiple choice options (see section 2, Highest priority)
* User selects answer
* Shows immediate feedback (correct/incorrect)
* Auto-advances to next question after 2 seconds (or user taps next)
* Shows summary screen after all questions in session
* Redirects to Home if user is not set in cookie

**Add Vocab Screen**
* Form to add one or more vocab items for practice
* Fields per item: source word, target word, category (dropdown or multi-select)
* Support bulk entry (e.g., paste CSV format with columns: source | target | category)
* Validation: all fields required, no duplicates allowed
* Success message on save
* Option to add another or return to Home

---

## 4. Spaced Repetition Algorithm Specification

### Algorithm Parameters (Configurable via `server/config.{yaml,json5}` and optional UI)

```
# Repetition thresholds
correct_streak_threshold: 5       # After N correct answers, deprioritize
incorrect_boost_threshold: 2      # After N incorrect answers, prioritize

# Time-based decay (in days)
deprioritize_duration: 3          # Days to deprioritize after correct_streak_threshold
prioritize_duration: 2            # Days to prioritize after incorrect_boost_threshold
min_interval_before_retest: 1     # Days before a word can be tested again after correct answer

# Scoring parameters
incorrect_weight: 3.0             # Multiplier for incorrect answers when calculating priority
time_decay_factor: 0.5            # Decay rate per day for recency bonus
base_priority: 0                  # Default priority for new vocab
```

### Algorithm Logic

1. **Calculate Priority Score for Each Word:**
   ```
   priority = base_priority
   
   // Recency bonus: prioritize words not recently practiced
   days_since_correct = (today - last_correct_date)
   if days_since_correct > min_interval_before_retest:
     priority += days_since_correct * time_decay_factor
   
   // Incorrect answer boost: prioritize frequently wrong answers
   priority += (total_incorrect_count * incorrect_weight)
   
   // Correct streak penalty: deprioritize well-known words
   if correct_streak >= correct_streak_threshold:
     priority -= deprioritize_duration * 10  // Tune this multiplier
   
   return priority
   ```

2. **Question Generation:**
   * Fetch top-N words (e.g., top 10) sorted by priority score (descending)
   * Shuffle order for variety
   * Generate practice session with these words
   * Per practice session: 10-20 questions (configurable)

3. **Hardness Tuning:**
   * **Lenient Mode:** Increase `deprioritize_duration`, decrease `incorrect_weight`
   * **Harsh Mode:** Decrease `deprioritize_duration`, increase `incorrect_weight`
   * Option to toggle via UI settings or config file

### Data Tracked Per Word Per User

* `total_correct_count`: Total times answered correctly (lifetime)
* `total_incorrect_count`: Total times answered incorrectly (lifetime)
* `current_correct_streak`: Consecutive correct answers in current period
* `last_correct_date`: ISO 8601 timestamp of last correct practice
* `last_incorrect_date`: ISO 8601 timestamp of last incorrect practice
* `created_at`: ISO 8601 timestamp when word was added
* `updated_at`: ISO 8601 timestamp of last interaction

---

## 5. API Response Contracts & Data Models

### Authentication & Authorization
* **No authentication required** (local single-machine app)
* Users identified by name cookie (`current_user`)
* All users in the app share the same vocab and categories (no per-user private content)

### HTTP Status Codes
* `200 OK`: Success
* `201 Created`: Resource created successfully
* `400 Bad Request`: Validation error (details in response body)
* `404 Not Found`: Resource not found
* `409 Conflict`: Duplicate entry or constraint violation
* `500 Internal Server Error`: Server error

### Response Format (JSON)

**Success Response:**
```json
{
  "status": "success",
  "data": { /* resource or array */ },
  "meta": { "timestamp": "2026-08-11T17:56:16Z" }
}
```

**Error Response:**
```json
{
  "status": "error",
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "User-friendly error message",
    "details": [
      { "field": "source_word", "reason": "Cannot be empty" }
    ]
  },
  "meta": { "timestamp": "2026-08-11T17:56:16Z" }
}
```

### Core Data Models

#### User
```
{
  "id": 1,                          // Auto-increment ID
  "name": "George",                 // Unique username
  "created_at": "2026-01-01T00:00:00Z",
  "updated_at": "2026-08-11T17:56:16Z"
}
```

#### Category
```
{
  "id": 1,
  "name": "Fruits",
  "created_at": "2026-01-01T00:00:00Z",
  "updated_at": "2026-08-11T17:56:16Z"
}
```

#### Vocab Word
```
{
  "id": 1,
  "source_language": "es",          // ISO 639-1 language code
  "source_word": "manzana",
  "target_language": "en",
  "target_word": "apple",
  "categories": [1, 3],             // Array of category IDs
  "created_at": "2026-01-01T00:00:00Z",
  "updated_at": "2026-08-11T17:56:16Z"
}
```

#### User Progress (Per Word)
```
{
  "id": 1,
  "user_id": 1,
  "word_id": 1,
  "total_correct_count": 5,
  "total_incorrect_count": 0,
  "current_correct_streak": 5,
  "last_correct_date": "2026-08-10T12:00:00Z",
  "last_incorrect_date": null,
  "created_at": "2026-01-02T00:00:00Z",
  "updated_at": "2026-08-10T12:00:00Z"
}
```

#### Practice Session
```
{
  "id": 1,
  "user_id": 1,
  "category_id": 1,
  "started_at": "2026-08-11T17:00:00Z",
  "completed_at": "2026-08-11T17:15:00Z",
  "questions_count": 10,
  "correct_count": 8,
  "accuracy": 0.8                   // 0.0 to 1.0
}
```

#### Practice Question Result
```
{
  "id": 1,
  "session_id": 1,
  "word_id": 1,
  "user_answer_id": 3,              // ID of selected option
  "correct_answer_id": 3,
  "is_correct": true,
  "answered_at": "2026-08-11T17:01:30Z"
}
```

---

## 6. REST API Routes

### Users

| Method | Route | Description | Request Body | Response |
|--------|-------|-------------|--------------|----------|
| POST | `/api/users` | Create user | `{ "name": "George" }` | `{ "status": "success", "data": { User } }` |
| GET | `/api/users` | List all users | — | `{ "status": "success", "data": [ User ] }` |
| GET | `/api/users/:id` | Get user by ID | — | `{ "status": "success", "data": { User } }` |

### Categories

| Method | Route | Description | Request Body | Response |
|--------|-------|-------------|--------------|----------|
| POST | `/api/categories` | Create category | `{ "name": "Fruits" }` | `{ "status": "success", "data": { Category } }` |
| GET | `/api/categories` | List all categories | — | `{ "status": "success", "data": [ Category ] }` |
| GET | `/api/categories/:id` | Get category by ID | — | `{ "status": "success", "data": { Category } }` |

### Vocabulary

| Method | Route | Description | Request Body | Response |
|--------|-------|-------------|--------------|----------|
| POST | `/api/vocab` | Add single vocab word | `{ "source_language": "es", "source_word": "manzana", "target_language": "en", "target_word": "apple", "categories": [1] }` | `{ "status": "success", "data": { Vocab } }` |
| POST | `/api/vocab/bulk` | Add multiple vocab words | `{ "words": [ { ...vocab }, ...] }` | `{ "status": "success", "data": { created: 10, failed: 0 } }` |
| GET | `/api/vocab?category_id=1` | List vocab (optionally filtered by category) | — | `{ "status": "success", "data": [ Vocab ], "meta": { "count": 50 } }` |
| GET | `/api/vocab/:id` | Get vocab by ID | — | `{ "status": "success", "data": { Vocab } }` |
| DELETE | `/api/vocab/:id` | Delete vocab word | — | `{ "status": "success", "data": null }` |

### Practice Sessions

| Method | Route | Description | Request Body | Response |
|--------|-------|-------------|--------------|----------|
| POST | `/api/practice/sessions` | Start practice session | `{ "user_id": 1, "category_id": 1, "question_count": 10 }` | `{ "status": "success", "data": { session_id: 1, questions: [ { id, word, options } ] } }` |
| GET | `/api/practice/sessions/:id` | Get session details | — | `{ "status": "success", "data": { Practice Session } }` |
| POST | `/api/practice/sessions/:id/answer` | Submit answer to question | `{ "question_id": 1, "selected_option_id": 3 }` | `{ "status": "success", "data": { is_correct: true, next_question: { ... } } }` |
| POST | `/api/practice/sessions/:id/complete` | Mark session complete | — | `{ "status": "success", "data": { summary: { correct, total, accuracy } } }` |

### User Progress

| Method | Route | Description | Response |
|--------|-------|-------------|----------|
| GET | `/api/users/:user_id/progress?category_id=1` | Get progress for user in category | `{ "status": "success", "data": { words: [ { word, total_correct, total_incorrect, ... } ] } }` |
| GET | `/api/users/:user_id/stats` | Get overall user stats | `{ "status": "success", "data": { total_words: 100, total_sessions: 50, avg_accuracy: 0.75 } }` |

---

## 7. User Stories & API Usage Examples

### Story 1: User George adds Spanish vocab
```
1. POST /api/users → create user "George"
2. POST /api/categories → create category "Fruits"
3. POST /api/vocab → add {"source_word": "manzana", "target_word": "apple", "categories": [1]}
   Response includes vocab ID
4. POST /api/vocab → add {"source_word": "naranja", "target_word": "orange", "categories": [1]}
```

### Story 2: User George practices and gets questions wrong
```
1. POST /api/practice/sessions → {"user_id": 1, "category_id": 1, "question_count": 10}
   Response: { session_id: 1, questions: [
     { id: 1, word: "manzana", options: ["apple", "orange", "banana", "grape", "lemon"] }
   ]}
2. POST /api/practice/sessions/1/answer → {"question_id": 1, "selected_option_id": 2}
   Response: { is_correct: false, next_question: {...} }
3. (Repeat step 2 for remaining questions)
4. POST /api/practice/sessions/1/complete
   Response: { summary: { correct: 7, total: 10, accuracy: 0.7 } }
   Backend: Update user_progress for each word in session
```

### Story 3: Check progress and verify prioritization algorithm
```
1. GET /api/users/1/progress?category_id=1
   Response: [
     { word_id: 1, source: "manzana", total_correct: 5, total_incorrect: 0, current_streak: 5, ... },
     { word_id: 2, source: "naranja", total_correct: 0, total_incorrect: 2, current_streak: 0, ... }
   ]
   Note: "naranja" should be prioritized for next practice session due to incorrect_boost_threshold=2
```

---

## 8. Error Handling & Edge Cases

### Input Validation
* **Empty fields:** All required fields must be non-empty strings or valid IDs
  * Response: `400 Bad Request` with details listing which field is invalid
* **Duplicate vocab:** Cannot add the same (source_word, target_word) pair to a category
  * Response: `409 Conflict` with message: "Vocab word already exists in this category"
* **Missing category:** When adding vocab, category IDs must exist
  * Response: `400 Bad Request` with message: "Category not found"
* **Invalid language codes:** source_language and target_language must be valid ISO 639-1 codes
  * Response: `400 Bad Request` with message: "Invalid language code"

### User & Session Management
* **User not found:** Attempting to access /api/users/:id with non-existent ID
  * Response: `404 Not Found`
* **Concurrent sessions:** Only one active practice session per user at a time
  * Response: `409 Conflict` if user tries to start session while one is active
* **Missing cookie:** Client requests Practice or Choose Category screens without current_user cookie
  * Response: Redirect to Home screen (client-side handling)

### Category & Vocab Deletion
* **Delete category with associated vocab:** Allow deletion but do NOT cascade-delete vocab words
  * Vocab words remain in app, but are no longer associated with that category
  * Update user_progress records to handle orphaned word IDs gracefully
* **Delete vocab word:** Remove from all categories, archive progress records (do not delete)
  * Response: `200 OK` with message: "Vocab deleted, progress records archived"

### Data Consistency
* **Session incomplete:** If user closes Practice screen mid-session
  * Mark session as abandoned (not completed) in DB
  * Do NOT update user_progress for abandoned questions
  * Response: Session remains in DB for auditing but does not count toward stats
* **Offline handling (future):** Client should cache questions locally; sync when online
  * Store session in localStorage if server is unavailable
  * Retry POST /api/practice/sessions/*/answer on reconnection

### Spaced Repetition Edge Cases
* **New user with empty vocab:** GET /api/practice/sessions → start returns session with 0 questions
  * Response: `200 OK` with empty questions array
  * Client shows message: "Add vocab first using Add Vocab screen"
* **All words deprioritized:** Spaced repetition algorithm deprioritizes all words for a category
  * Fallback: Return words sorted by created_at (oldest first) for review
  * Response: Normal session response, but note in logs for debugging
* **Unchaned config during app lifetime:** Config changes require server restart
  * Document in README that tuning parameters require restart

---

## 9. Database Schema (Plaintext Diagram)

```
┌─────────────────┐
│     users       │
├─────────────────┤
│ id (PK)         │
│ name (UNIQUE)   │
│ created_at      │
│ updated_at      │
└────────┬────────┘
         │
         │ 1:M
         ▼
┌─────────────────────────────┐
│   user_progress             │
├─────────────────────────────┤
│ id (PK)                     │
│ user_id (FK)                │
│ word_id (FK)                │
│ total_correct_count         │
│ total_incorrect_count       │
│ current_correct_streak      │
│ last_correct_date           │
│ last_incorrect_date         │
│ created_at                  │
│ updated_at                  │
│ UNIQUE(user_id, word_id)    │
└─────────────────────────────┘

┌──────────────────┐
│   categories     │
├──────────────────┤
│ id (PK)          │
│ name             │
│ created_at       │
│ updated_at       │
└────────┬─────────┘
         │
         │ M:M (junction table)
         ▼
    vocab_categories
         │
         │ 1:M
         ▼
┌──────────────────────────────┐
│   vocab_words                │
├──────────────────────────────┤
│ id (PK)                      │
│ source_language (ISO 639-1)  │
│ source_word                  │
│ target_language (ISO 639-1)  │
│ target_word                  │
│ created_at                   │
│ updated_at                   │
└────────┬─────────────────────┘
         │
         │ 1:M
         ▼
┌──────────────────────────────────┐
│   practice_sessions              │
├──────────────────────────────────┤
│ id (PK)                          │
│ user_id (FK)                     │
│ category_id (FK)                 │
│ started_at                       │
│ completed_at (nullable)          │
│ questions_count                  │
│ correct_count                    │
│ accuracy (0.0 - 1.0)             │
│ created_at                       │
│ updated_at                       │
└────────┬─────────────────────────┘
         │
         │ 1:M
         ▼
┌──────────────────────────────────┐
│   practice_question_results      │
├──────────────────────────────────┤
│ id (PK)                          │
│ session_id (FK)                  │
│ word_id (FK)                     │
│ user_answer_id (option ID)       │
│ correct_answer_id (option ID)    │
│ is_correct (boolean)             │
│ answered_at                      │
└──────────────────────────────────┘

┌──────────────────┐
│   config         │
├──────────────────┤
│ key (PK)         │
│ value (JSON)     │
│ updated_at       │
└──────────────────┘
(Stores spaced repetition tuning parameters)
```

---

## 10. Development Strategy
* Enable early manual user testing by prioritizing completion end-to-end features, feature-by-feature if possible.
* Define a dockerfile per service, and a docker-compose file at the repo root so that a developer can run the app for manual testing with `docker compose up`
* **Milestone 1:** User + Category + Vocab Add (MVP data layer)
* **Milestone 2:** Multiple Choice Practice with basic progress tracking
* **Milestone 3:** Spaced repetition algorithm implementation
* **Milestone 4:** UI polish and performance optimization
* **Milestone 5 (Deferred):** Text-based practice mode

---

## Notes for Agent Implementation
* All timestamps must be ISO 8601 format (UTC)
* Language codes must follow ISO 639-1 standard (e.g., "en", "es", "fr")
* Database migrations should be numbered and versioned
* Ensure all API routes validate user_id exists before processing
* For practice sessions, pre-generate all questions at session start (avoid dynamic question generation mid-session for consistency)
* Consider seeding test data (5-10 users, 3-5 categories, 20-50 vocab words) for manual testing
