# Enhanced Language-Practice Flashcards

Status: ready-for-agent

## Problem Statement

Learners need a local, shared language-practice application that lets multiple named people build vocabulary, practice translations, and see meaningful progress over time. The current repository does not yet provide the client, server, persistence model, practice workflow, or spaced-repetition behavior needed to support that experience.

The application must work well on an iPhone in Safari, remain simple to run on one machine, preserve learning history accurately, and make the practice algorithm tunable without requiring code changes.

## Solution

Build a local flashcards application with a TypeScript/React client and a Rust/axum server backed initially by SQLite. Learners use durable local profiles selected through the Home screen. Vocabulary Entries and Categories are shared across all Learners, while Direction-specific Progress belongs to one Learner and one Vocabulary Entry in one Translation Direction.

Learners create and organize Vocabulary Entries, choose a Category and Translation Direction, and start a Practice Session. Each session snapshots its complete question content and options at creation. Every question displays four translation options plus an explicit “Don’t know” response. A temporary selection is not scored; only one explicit Answer Submission is accepted. Progress changes transactionally when the answer is submitted, and the session completes automatically after every question has been answered.

Spaced repetition selects Eligible Entries using each Learner’s persisted Learner Algorithm Settings. Retest cooldown is a hard eligibility rule. Incorrect answers, including “Don’t know,” increase priority and reset the current direction-specific streak; correct answers increase the streak and may reduce future priority after the configured threshold.

## User Stories

1. As a new Learner, I want to create a durable profile with a unique display name, so that my progress survives browser restarts.
2. As an existing Learner, I want to select my profile from Home, so that I can continue practicing with my own progress.
3. As a Learner, I want the app to remember my selected profile in a secure cookie, so that I do not need to identify myself on every screen.
4. As a Learner, I want an invalid or deleted profile cookie to be cleared and redirected to Home, so that stale identity state cannot access another profile.
5. As a Learner, I want to rename my profile while preserving its durable identity and progress, so that display-name corrections do not create a new learner.
6. As a Learner, I want learner names to be unique regardless of case or surrounding whitespace, so that profiles cannot be confused.
7. As a Learner, I want to delete my profile deliberately, so that my personal progress and sessions can be removed from the local app.
8. As a Learner, I want to create a Category, so that I can organize shared learning content.
9. As a Learner, I want to view Categories sorted by configured creation-date or alphabetical order, so that I can find a practice topic quickly.
10. As a Learner, I want to rename a Category, so that shared organization can be corrected without recreating content.
11. As a Learner, I want to delete a Category when it will not orphan a Vocabulary Entry, so that shared content always remains valid.
12. As a Learner, I want category deletion to reject unsafe operations, so that shared Vocabulary Entries are never silently deleted.
13. As a Learner, I want to create a Vocabulary Entry containing source text, target text, and both language codes, so that I can practice words and short phrases.
14. As a Learner, I want Vocabulary Entries to support arbitrary short text rather than only single words, so that phrases and articles can be learned.
15. As a Learner, I want duplicate language-and-text pairs rejected globally, so that one shared entry has one consistent learning identity.
16. As a Learner, I want every Vocabulary Entry to belong to at least one Category, so that all content is discoverable and practiceable.
17. As a Learner, I want one Vocabulary Entry to belong to many Categories, so that content can be practiced under different topics.
18. As a Learner, I want to add several Categories to a Vocabulary Entry during creation, so that organization is correct from the beginning.
19. As a Learner, I want to edit Vocabulary Entry text and Category Memberships, so that mistakes can be corrected without losing the entry’s identity.
20. As a Learner, I want language codes to be normalized and validated against ISO 639-1, so that language-pair behavior is reliable.
21. As a Learner, I want Vocabulary Entry languages to remain immutable after creation, so that changing a language does not silently change the meaning of existing progress.
22. As a Learner, I want to delete a Vocabulary Entry deliberately, so that obsolete content disappears from future practice.
23. As a Learner, I want deleting a Vocabulary Entry to remove its Category Memberships, Direction-specific Progress, and practice history, so that current statistics do not retain deleted content.
24. As a Learner, I want to paste CSV-style vocabulary data into the client, so that bulk entry is efficient.
25. As a Learner, I want bulk import to resolve existing Categories by normalized name, so that imported rows attach to the intended shared content.
26. As a Learner, I want an invalid or duplicate bulk row to reject the whole import, so that bulk entry is atomic and predictable.
27. As a Learner, I want to see each Category’s proficiency for the selected profile, so that I can choose what to practice next.
28. As a Learner, I want proficiency shown separately for each Translation Direction, so that reverse-direction weaknesses are visible.
29. As a Learner, I want an overall Category proficiency summary in addition to direction-specific values, so that I can scan my general progress.
30. As a Learner, I want to choose a Category before starting practice, so that a session focuses on a meaningful topic.
31. As a Learner, I want to choose one Translation Direction per session, so that I know which knowledge direction is being tested.
32. As a Learner, I want the server to use my cookie identity for learner-scoped operations, so that a request cannot impersonate another Learner by supplying an arbitrary ID.
33. As a Learner, I want the app to generate a bounded number of questions, so that sessions fit a short mobile practice activity.
34. As a Learner, I want requested question counts validated against server-configured minimum and maximum values, so that sessions cannot be empty or unreasonably large.
35. As a Learner, I want the app to prefer Eligible Entries from my selected Category and Language Pair, so that distractors are relevant.
36. As a Learner, I want distractors to fall back to the same Language Pair across other Categories when necessary, so that valid questions can still be generated.
37. As a Learner, I want entries without enough distinct distractors omitted, so that the app never repeats or invents misleading options.
38. As a Learner, I want a session to contain the available valid questions when fewer than requested can be generated, so that practice remains useful.
39. As a Learner, I want a zero-question session prevented, so that the app clearly tells me when no practice is available.
40. As a Learner, I want all questions generated at session start, so that later content edits do not alter an active session.
41. As a Learner, I want each question to preserve the displayed source, target, direction, and options, so that historical answers reflect what I actually saw.
42. As a Learner, I want four translation options plus “Don’t know,” so that every question offers a safe explicit response.
43. As a Learner, I want temporary option selection to remain unscored until I submit, so that changing my mind does not create false attempts.
44. As a Learner, I want each question to accept only one explicit Answer Submission, so that retries cannot inflate progress counts.
45. As a Learner, I want “Don’t know” to count as an explicit incorrect response, so that uncertainty feeds the learning algorithm.
46. As a Learner, I want immediate correct/incorrect feedback after submission, so that I can learn from each answer.
47. As a Learner, I want the next question available after feedback, so that the session flow is quick on mobile.
48. As a Learner, I want a Practice Session to complete automatically after every question is answered, so that incomplete sessions cannot be reported as completed.
49. As a Learner, I want an idempotent completion confirmation endpoint, so that clients can safely finalize already completed sessions.
50. As a Learner, I want an active session to be discarded when I restart, so that I can begin a fresh question set without being blocked.
51. As a Learner, I want an active session to be discarded after inactivity timeout, so that a forgotten session cannot block future practice.
52. As a Learner, I want only one active Practice Session at a time, so that progress and navigation remain unambiguous.
53. As a Learner, I want closing or restarting an unfinished session to discard it, so that incomplete activity does not pollute session statistics.
54. As a Learner, I want correct submissions to increment total correct count and current streak for the selected direction, so that demonstrated knowledge is recorded.
55. As a Learner, I want incorrect submissions to increment total incorrect count, reset the selected direction’s streak, and record the latest incorrect time, so that difficulty is reflected accurately.
56. As a Learner, I want progress tracked separately for each Translation Direction, so that knowing one direction does not imply knowing the reverse.
57. As a Learner, I want progress rows created lazily, so that new content and new profiles do not create unnecessary records.
58. As a Learner, I want the practice algorithm to prioritize directions with repeated incorrect answers, so that difficult material returns for review.
59. As a Learner, I want the algorithm to prioritize directions that have not been correctly practiced recently, so that review does not stop permanently.
60. As a Learner, I want a correct streak threshold to reduce priority for familiar material, so that practice time focuses on weaker areas.
61. As a Learner, I want retest cooldown to be a hard exclusion after a correct answer, so that recently mastered material is not immediately repeated.
62. As a Learner, I want all priority and cooldown calculations based on exact elapsed UTC durations, so that time behavior is consistent across daylight-saving changes.
63. As a Learner, I want my algorithm parameters stored independently, so that my preferred practice difficulty does not change another Learner’s experience.
64. As a Learner, I want my initial settings copied from server defaults, so that the app has sensible behavior without requiring configuration first.
65. As a Learner, I want to edit and reset my Learner Algorithm Settings in a dedicated settings surface, so that tuning is understandable and persistent.
66. As a Learner, I want valid settings accepted even when they produce no Eligible Entries, so that the server does not silently override my choices.
67. As a Learner, I want current progress statistics to exclude deleted Vocabulary Entries, so that totals describe only current shared content.
68. As a Learner, I want completed sessions to contribute to statistics while discarded sessions do not, so that summaries reflect finished learning activity.
69. As a Learner, I want aggregate statistics visible across Learners, so that the shared app can show overall activity.
70. As a Learner, I want detailed Direction-specific Progress limited to the selected Learner, so that shared visibility does not expose another person’s learning history unnecessarily.
71. As a Learner, I want consistent JSON success and error envelopes, so that the client can handle all endpoints uniformly.
72. As a Learner, I want empty response fields omitted rather than serialized as `null`, so that responses remain concise and semantically clear.
73. As a Learner, I want validation errors to identify invalid fields, so that I can correct content quickly.
74. As a Learner, I want duplicate and constraint conflicts reported distinctly, so that I know whether to edit or retry.
75. As a developer, I want database and migration paths configurable, so that local and containerized deployments can use different storage locations.
76. As a developer, I want the client and server runnable as separate Podman services, so that each boundary can evolve independently.
77. As a developer, I want SQLite persisted through a shared container volume, so that local practice data survives restarts.
78. As a developer, I want application defaults such as language list, session limits, timeout, cookie lifetime, and algorithm defaults in YAML configuration, so that deployment behavior is adjustable without code changes.
79. As a developer, I want raw SQL and explicit transactions for core persistence operations, so that data consistency is visible and predictable.
80. As a developer, I want migrations numbered and versioned, so that database setup is repeatable.

## Implementation Decisions

- Build the first implementation for Milestones 1–3: shared Learners, Categories, Vocabulary Entries, multiple-choice Practice Sessions, progress tracking, and spaced repetition.
- Use a TypeScript/React client optimized for iPhone Safari and a Rust server using axum.
- Use SQLite initially, while keeping the database URL configurable.
- Use raw SQL for persistence and explicit database transactions for answer submission, content deletion, and other multi-record operations.
- Use a YAML configuration file with configurable database URL, migration path, supported languages, question-count bounds, session inactivity timeout, cookie lifetime, and application-level algorithm defaults.
- Initialize each Learner’s complete Learner Algorithm Settings from application defaults. Persist later changes per Learner; stored settings override changed file defaults for existing Learners.
- Treat a Vocabulary Entry as one language-neutral translation pair with arbitrary short source and target text. Store one unordered Language Pair and select a Translation Direction per Practice Session.
- Model alternate translations as separate Vocabulary Entries rather than multiple accepted targets on one entry.
- Normalize surrounding whitespace and Unicode form for duplicate checks and input identity while preserving case, punctuation, and accents as meaningful content.
- Compare Learner and Category names case-insensitively after trimming whitespace, while preserving display casing.
- Validate and normalize language codes to lowercase ISO 639-1 values from a maintained supported-language list.
- Make Learner profiles durable, uniquely named, renameable, and independently progress-tracked. Use the durable Learner ID as the current-learner cookie value.
- Use a host-only, `HttpOnly`, `SameSite=Lax` cookie with an explicit long expiry. Clear it and redirect to Home when the identity is invalid.
- Make Categories and Vocabulary Entries shared content that every local Learner may create, edit, or delete.
- Require every Vocabulary Entry to have at least one Category Membership. Allow many-to-many membership.
- Reject Category deletion when it would remove the final membership of any Vocabulary Entry. Discard active sessions affected by a deleted Category; completed snapshots remain historical until their Vocabulary Entries are deleted.
- Permit Vocabulary Entry text and Category Membership edits, but keep source and target languages immutable after creation.
- Define Vocabulary Deletion as removal of the entry, all Category Memberships, all Direction-specific Progress, and all practice history associated with that entry. Recompute current statistics without deleted content.
- Accept JSON for the bulk API. Parse the UI’s `source | target | category` CSV convenience format in the client, resolve Categories by normalized name, and make the complete import atomic.
- Track Direction-specific Progress lazily per Learner, Vocabulary Entry, and Translation Direction.
- Generate a Practice Session for one Learner, one Category, and one Translation Direction. Enforce one active session per Learner globally.
- Require a server-configured question-count range, defaulting to 10–20. Allow a session with fewer questions than requested when valid questions are available; create no session when zero questions are available.
- Select Eligible Entries using Category-first distractor sourcing and same-Language-Pair fallback. Omit entries that cannot produce four distinct incorrect translation options.
- Display exactly four translation options plus a “Don’t know” response. The correct option must exactly equal the stored target text for the selected direction.
- Snapshot all displayed question content, direction, distractor text, option identity, and ordering when the session starts. Content edits and deletions must not rewrite completed historical snapshots.
- Keep temporary UI selection unscored. Require one explicit Answer Submission per question and make duplicate submissions idempotent.
- Treat “Don’t know” as an explicit incorrect Answer Submission.
- Update question result, Direction-specific Progress, and Practice Session counters in one transaction.
- Increment correct totals and the current direction-specific streak on correct submissions. Increment incorrect totals, reset the current direction-specific streak, and update the latest incorrect timestamp on incorrect submissions.
- Complete a session automatically after all generated questions receive one Answer Submission. Keep completion confirmation idempotent. Discard unfinished sessions on restart, timeout, or leaving the flow; discarded sessions do not contribute to statistics.
- Use a hard retest cooldown after correct practice. Rank eligible directions using configurable base priority, elapsed-time recency bonus, incorrect-answer weight, and correct-streak penalty.
- Calculate time-based behavior with exact elapsed UTC durations rather than local calendar dates.
- Expose detailed progress only for the selected Learner. Expose aggregate statistics across Learners using only retained Vocabulary Entries and completed sessions.
- Provide a dedicated Learner settings surface/API for algorithm tuning and reset-to-default behavior.
- Use a stable JSON envelope for all success and error responses. Successful deletion uses an envelope without an empty `null` field; optional empty fields are omitted. Errors contain machine-readable codes, user-facing messages, and field-level details where relevant.
- Use standard HTTP statuses for success, validation, missing resources, conflicts, and server failures.
- Provide user, Category, Vocabulary Entry, Practice Session, progress, statistics, and Learner settings API operations. Learner-scoped operations derive identity from the cookie rather than trusting redundant request-body user IDs.
- Run separate client and server Podman services and persist SQLite through a shared volume.

## Testing Decisions

- Test external behavior at the highest confirmed seam: black-box HTTP integration tests against the axum API using an isolated SQLite database.
- Exercise the complete flows through HTTP: profile selection, shared content creation and editing, atomic bulk import, Category and Vocabulary Deletion, session generation, answer submission, automatic completion, restart/timeout discard, progress, statistics, and settings.
- Assert response status, JSON envelope shape, omitted empty fields, validation details, conflict behavior, cookie behavior, and redirect behavior.
- Verify database-visible outcomes through subsequent API behavior rather than coupling tests to internal module structure.
- Cover domain edge cases through API scenarios: duplicate normalized names, duplicate Vocabulary Entries, missing Category Memberships, unsafe Category deletion, insufficient distractors, category-first fallback, hard cooldown exclusion, incorrect-answer boosts, streak penalties, direction isolation, idempotent submissions, duplicate completion, invalid cookies, and deleted content.
- Verify transactional behavior by forcing invalid answer/content operations and confirming that no partial progress, session counters, or shared-content changes are observable afterward.
- Verify immutable snapshots by editing or deleting shared content after a session is created and confirming the session’s displayed and historical question content remains stable.
- Verify aggregate statistics exclude discarded sessions and deleted Vocabulary Entries while selected-Learner detailed progress remains correctly isolated.
- Add focused deterministic tests around the priority calculation only where its time and threshold edge cases are clearer and less brittle than full HTTP setup; these tests should still assert behavior, not implementation details.
- Existing repository test prior art is absent because the client/server implementation is not yet present. Establish the HTTP integration harness as the project’s first testing convention, with focused algorithm tests as a narrow complement.

## Out of Scope

- Text-based typed practice and typed-answer normalization/acceptance rules.
- Offline localStorage session caching and synchronization.
- Production authentication, authorization, remote multi-tenant identity, or administrator roles.
- Multiple-choice synonym acceptance; correctness is exact equality with the curated target text.
- Multiple target translations on a single Vocabulary Entry.
- Cross-device synchronization and hosted deployment.
- Advanced UI polish, analytics, and performance optimization beyond the behavior needed for a usable iPhone Safari flow.
- Supporting a second database engine in the first implementation.
- Server-side CSV parsing.
- Persistent abandoned-session audit records; unfinished sessions are discarded.

## Further Notes

- Historical session snapshots are intentionally separate from current shared content: edits affect current Vocabulary Entries and progress views, while completed snapshots preserve what the Learner saw.
- A Vocabulary Entry must always remain categorized. This invariant is why unsafe Category deletion is rejected rather than cascading into shared content deletion.
- The current domain glossary in `CONTEXT.md` is authoritative for terminology. In particular, use “Vocabulary Entry,” “Learner,” “Practice Session,” “Answer Submission,” “Don't Know Response,” “Direction-specific Progress,” and “Learner Algorithm Settings” instead of older ambiguous terms such as “word,” “user progress,” or “quiz.”
- The implementation should provide clear user-facing messaging when no Eligible Entries exist, when a session has fewer questions than requested, and when content operations are rejected by an invariant.
- The server must document that YAML application defaults apply to new Learners and that persisted Learner Algorithm Settings govern existing Learners until explicitly changed or reset.
