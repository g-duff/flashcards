# 09 — Session restart, one-active-session rule & inactivity discard

**What to build:** A Learner never gets stuck with a stale or ambiguous session: restarting discards the current one and starts fresh, only one active session can exist per Learner at a time, and an abandoned session times out and disappears without polluting statistics.

**Blocked by:** 08 — Answer submission with transactional progress tracking & auto-completion.

**Status:** ready-for-agent

- [ ] `POST /api/practice-sessions/:id/restart` discards the active session and generates a replacement under the same one-active-session rule.
- [ ] Attempting to create a second active session for a Learner without restarting returns `409`.
- [ ] An active session is discarded after the configured inactivity timeout (`session_inactivity_timeout_minutes`).
- [ ] Discarded sessions are removed rather than retained as historical activity, and do not contribute to statistics.
- [ ] Client offers restart from the Practice/Choose-Category flow and handles the resulting new session.
- [ ] HTTP integration tests cover: explicit restart discarding + replacing, one-active-session `409` conflict, inactivity-timeout discard, discarded sessions excluded from subsequent statistics queries.
