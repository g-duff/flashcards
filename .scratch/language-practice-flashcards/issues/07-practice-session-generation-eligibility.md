# 07 — Practice Session generation & eligibility

**What to build:** A Learner can choose a Category, Translation Direction, and question count, and start a Practice Session whose questions are generated once, immutably snapshotted, and built only from entries that can produce four genuine distractors.

**Blocked by:** 05 — Vocabulary Entry CRUD (single).

**Status:** resolved

- [x] `POST /api/practice-sessions` creates an active session for the current Learner given `category_id`, `direction`, and `question_count`.
- [x] Requested question count is validated against server-configured min/max bounds (`400` if out of range).
- [x] An entry+direction is Eligible only when it belongs to the selected Category, is outside its hard retest cooldown after the latest correct answer, and the Language Pair has enough distinct entries to build four incorrect distractors.
- [x] Distractors are sourced from the selected Category first, falling back to other Categories in the same Language Pair when needed; entries that still can't produce four distinct incorrect options are omitted from the session.
- [x] If fewer valid questions exist than requested, the session is created with the available count; if zero are available, no session is created and the response explains why (no `500`, a clear user-facing message).
- [x] At creation, the server snapshots every question's prompt text, correct answer text, direction, distractor texts, option identities, and ordering; the public response never reveals `is_correct` before submission.
- [x] `GET /api/practice-sessions/:id` reads the session snapshot and status.
- [x] Client "Choose Practice Category" screen lets the Learner pick Category, Direction, and question count, and start a session; client "Practice" screen renders the prompt and four options plus "Don't know" without exposing the answer.
- [x] HTTP integration tests cover: session creation, question-count bounds rejection, category-first distractor sourcing, language-pair fallback, entry omission for insufficient distractors, short session when fewer than requested are available, zero-eligible rejection, snapshot shape hiding correctness pre-submission.

**Resolution note:** Per explicit clarification, eligibility requires four *incorrect* distractors (1 correct + 4 incorrect = 5 translation options, plus "Don't know" = 6 total options), resolving a contradiction between the Eligibility section ("four incorrect distractors") and the UI section ("four options plus Don't know") in the grilled spec.
