# 08 — Answer submission with transactional progress tracking & auto-completion

**What to build:** A Learner can answer each question in an active session exactly once (including an explicit "Don't know"), see immediate feedback, and have the session automatically complete after the last question — with Direction-specific Progress updated atomically alongside the answer.

**Blocked by:** 07 — Practice Session generation & eligibility.

**Status:** ready-for-agent

- [ ] `POST /api/practice-sessions/:id/questions/:question_id/submit` accepts one explicit Answer Submission (`option_id`, or a "Don't know" indication); temporary UI selection never reaches this endpoint as scored state.
- [ ] Repeating a submission for an already-answered question is idempotent: it returns the existing result without reapplying progress or counters.
- [ ] "Don't know" is treated as an explicit incorrect Answer Submission.
- [ ] Correctness is exact equality with the snapshot's stored correct text for the selected direction.
- [ ] The response includes correctness/feedback and the next unanswered question when one exists.
- [ ] Direction-specific Progress rows are created lazily on first submission for a given Learner/Vocabulary Entry/Direction. Correct submissions increment `total_correct_count` and `current_correct_streak`; incorrect submissions (including Don't know) increment `total_incorrect_count`, reset the streak, and update `last_incorrect_at` (correct updates `last_correct_at`).
- [ ] Question result, Direction-specific Progress, and Practice Session counters (answered/correct counts) update together in one database transaction; a forced failure leaves no partial state.
- [ ] The session transitions to `completed` automatically once every generated question has one Answer Submission — never by client request alone.
- [ ] `POST /api/practice-sessions/:id/complete` is an idempotent completion-confirmation endpoint; calling it before every question is answered is rejected.
- [ ] Client Practice screen shows progress (e.g. `5/20`), allows temporary selection before submit, shows immediate feedback, advances only after submission, and shows a summary after the final answer.
- [ ] HTTP integration tests cover: submission scoring, Don't know handling, duplicate-submission idempotency, transactional atomicity under a forced failure, lazy progress creation, streak/count updates, automatic completion, idempotent completion confirmation, premature-completion rejection.

## Deferred from ticket 04

Ticket 04 (Shared Category CRUD) required `GET /api/categories` to include
current-Learner proficiency per spec.md's route table (spec.md story
27–29). It was deferred because proficiency is computed from
Direction-specific Progress, which doesn't exist until this ticket. Ticket
05 re-deferred it here rather than picking it up, since Progress is still
absent at that point too.

- [ ] `GET /api/categories` includes current-Learner proficiency (overall and per Translation Direction, per spec.md story 27–29) now that Direction-specific Progress exists.
