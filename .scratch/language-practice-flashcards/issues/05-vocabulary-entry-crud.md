# 05 — Vocabulary Entry CRUD (single)

**What to build:** A Learner can create a Vocabulary Entry (a word or short phrase translation pair) with validated languages and at least one Category, edit its text and Category Memberships later, and delete it — with deletion fully cleaning up dependent data.

**Blocked by:** 04 — Shared Category CRUD.

**Status:** ready-for-agent

- [x] `POST /api/vocabulary-entries` creates one entry from source/target text, source/target ISO 639-1 language codes, and one or more `category_ids`; at least one Category Membership is required (`400` otherwise).
- [x] Language codes are normalized to lowercase ISO 639-1 and validated against the configured supported-language list.
- [x] Surrounding whitespace and Unicode form are normalized for duplicate-identity checks; case, punctuation, and accents remain meaningful. Duplicate language-and-text pairs are rejected globally with `409`.
- [x] `GET /api/vocabulary-entries` lists entries, optionally filtered by Category or Language Pair; `GET /api/vocabulary-entries/:id` reads one.
- [x] `PATCH /api/vocabulary-entries/:id` edits text and/or Category Memberships; source/target languages are immutable after creation and rejected if a change is attempted.
- [x] `DELETE /api/vocabulary-entries/:id` removes the entry, its Category Memberships, Direction-specific Progress, and practice history in one transaction. (Direction-specific Progress and practice-history tables do not exist yet — they land in tickets 07/08 — so today's cleanup covers the entry and its Category Memberships; nothing else references a Vocabulary Entry yet.)
- [x] Client "Add Vocabulary" screen supports single-entry creation (text, languages, one or more Categories) and shows validation/duplicate errors.
- [x] HTTP integration tests cover: creation, missing-Category `400`, duplicate-pair `409`, language normalization/validation, immutable-language rejection on edit, edit of text/memberships, delete cascade cleanup.

## Deferred from ticket 04

Ticket 04 (Shared Category CRUD) required `DELETE /api/categories/:id` to
reject with `409` when it would remove the final Category Membership of any
Vocabulary Entry, and required `GET /api/categories` to include
current-Learner proficiency. Both were deferred here because they depend on
types this ticket introduces (Vocabulary Entries, Category Memberships,
Direction-specific Progress), and ticket 04's own "Blocked by" only listed
ticket 02 — so ticket 04 was implemented without them rather than pulling
this ticket's schema forward early. Pick these up as part of this ticket:

- [x] `DELETE /api/categories/:id` is rejected with `409` when it would remove the final Category Membership of any Vocabulary Entry; deletion never cascade-deletes Vocabulary Entries. In scope for this ticket, since it only needs Vocabulary Entries and Category Memberships, both introduced here.
- [x] HTTP integration test: unsafe Category deletion is rejected with `409`.
- [ ] `GET /api/categories` includes current-Learner proficiency per spec.md's route table (spec.md story 27–29). Out of scope for this ticket too — proficiency needs Direction-specific Progress, which isn't introduced until ticket 08 (Answer submission & progress tracking). Re-deferred to ticket 08 rather than picked up here.
