# 12 — Category proficiency & statistics

**What to build:** A Learner can see how well they know each Category — overall and per Translation Direction — plus their own detailed progress and shared aggregate activity across all Learners, all computed only from current shared content and completed sessions.

**Blocked by:** 09 — Session restart, one-active-session rule & inactivity discard; 08 — Answer submission with transactional progress tracking & auto-completion.

**Status:** ready-for-agent

- [ ] `GET /api/categories` includes the current Learner's proficiency per Category: an overall summary value plus separate values per Translation Direction.
- [ ] `GET /api/me/progress` returns detailed Direction-specific Progress limited to the current Learner.
- [ ] `GET /api/me/stats` returns the current Learner's aggregate statistics; `GET /api/stats` returns aggregate statistics across all Learners.
- [ ] All statistics/proficiency views exclude deleted Vocabulary Entries and discarded sessions; only completed sessions and currently-retained content contribute.
- [ ] Detailed per-entry progress is never exposed for a Learner other than the current one.
- [ ] Client "Choose Practice Category" screen shows overall and direction-specific proficiency per Category.
- [ ] HTTP integration tests cover: overall + direction-specific proficiency values, detailed progress scoped to the current Learner only, current-Learner vs. cross-Learner aggregate stats, exclusion of deleted entries and discarded sessions from all of the above.
