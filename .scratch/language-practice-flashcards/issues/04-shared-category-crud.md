# 04 — Shared Category CRUD

**What to build:** Any Learner can create, browse, rename, and delete shared Categories, with deletion refusing to ever orphan a Vocabulary Entry.

**Blocked by:** 02 — Learner creation, selection & cookie identity.

**Status:** ready-for-agent

- [x] `POST /api/categories` creates a shared Category; `GET /api/categories` lists Categories, sortable by creation date or alphabetically. **Note:** spec.md's route table also describes this endpoint as returning "current-Learner proficiency" (spec.md story 27–29); proficiency depends on Direction-specific Progress, which doesn't exist until later tickets, so it is omitted here and left for the ticket that introduces Progress.
- [x] `GET /api/categories/:id` reads a single Category; `PATCH /api/categories/:id` renames it.
- [x] Category names are compared case-insensitively after trimming whitespace for uniqueness; duplicates return `409`.
- [~] `DELETE /api/categories/:id` is rejected with `409` when it would remove the final Category Membership of any Vocabulary Entry; deletion never cascade-deletes Vocabulary Entries. **Category Memberships and Vocabulary Entries don't exist yet (ticket 05)**, so deletion is currently unconditional (nothing can be orphaned); the orphan-check must be added alongside ticket 05.
- [x] Client screen lists Categories (sorted per the above) and supports create/rename/delete with clear messaging on rejected deletion.
- [~] HTTP integration tests cover: creation, duplicate-name conflict, listing/sort order, rename, safe deletion, unsafe-deletion rejection. **Unsafe-deletion rejection test deferred to ticket 05** for the same reason as above; safe deletion and all other cases are covered.
