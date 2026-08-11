# 05 — Vocabulary Entry CRUD (single)

**What to build:** A Learner can create a Vocabulary Entry (a word or short phrase translation pair) with validated languages and at least one Category, edit its text and Category Memberships later, and delete it — with deletion fully cleaning up dependent data.

**Blocked by:** 04 — Shared Category CRUD.

**Status:** ready-for-agent

- [ ] `POST /api/vocabulary-entries` creates one entry from source/target text, source/target ISO 639-1 language codes, and one or more `category_ids`; at least one Category Membership is required (`400` otherwise).
- [ ] Language codes are normalized to lowercase ISO 639-1 and validated against the configured supported-language list.
- [ ] Surrounding whitespace and Unicode form are normalized for duplicate-identity checks; case, punctuation, and accents remain meaningful. Duplicate language-and-text pairs are rejected globally with `409`.
- [ ] `GET /api/vocabulary-entries` lists entries, optionally filtered by Category or Language Pair; `GET /api/vocabulary-entries/:id` reads one.
- [ ] `PATCH /api/vocabulary-entries/:id` edits text and/or Category Memberships; source/target languages are immutable after creation and rejected if a change is attempted.
- [ ] `DELETE /api/vocabulary-entries/:id` removes the entry, its Category Memberships, Direction-specific Progress, and practice history in one transaction.
- [ ] Client "Add Vocabulary" screen supports single-entry creation (text, languages, one or more Categories) and shows validation/duplicate errors.
- [ ] HTTP integration tests cover: creation, missing-Category `400`, duplicate-pair `409`, language normalization/validation, immutable-language rejection on edit, edit of text/memberships, delete cascade cleanup.
