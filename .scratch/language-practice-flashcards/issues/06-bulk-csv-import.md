# 06 — Bulk CSV import

**What to build:** A Learner can paste `source | target | category` rows into the client and commit them as Vocabulary Entries in one atomic operation, with existing Categories resolved by normalized name.

**Blocked by:** 05 — Vocabulary Entry CRUD (single).

**Status:** ready-for-agent

- [ ] Client parses the pasted `source | target | category` CSV-style convenience format and sends it to the server as JSON (no server-side CSV parsing).
- [ ] `POST /api/vocabulary-entries/bulk` resolves each row's Category by normalized name and creates all entries atomically.
- [ ] Any invalid row (missing fields, unknown Category, bad language, or duplicate identity — including duplicates within the same batch) rejects the entire import with no partial writes.
- [ ] Client shows validation/duplicate errors clearly before or on commit.
- [ ] HTTP integration tests cover: successful atomic bulk create, unknown-Category rejection of the whole batch, invalid/duplicate-row rejection of the whole batch, verifying no partial rows persist after a forced failure.
