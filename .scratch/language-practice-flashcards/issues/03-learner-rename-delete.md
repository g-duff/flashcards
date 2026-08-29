# 03 — Learner rename & delete

**What to build:** A Learner can correct their display name without losing their identity or progress, and can deliberately delete their profile, removing their own data without touching shared content.

**Blocked by:** 02 — Learner creation, selection & cookie identity.

**Status:** done

- [x] `PATCH /api/learners/:id` renames a Learner, preserving durable ID and all existing progress; the new name is subject to the same case-insensitive, trimmed uniqueness check as creation.
- [x] `DELETE /api/learners/:id` removes the Learner and all personal dependent data (settings, progress, sessions, questions, submissions) but never shared Categories or Vocabulary Entries.
- [x] Deleting the current Learner clears the current-learner cookie.
- [x] HTTP integration tests cover: rename preserving identity/progress, rename uniqueness conflict, delete removing only personal data, post-delete cookie behavior.
