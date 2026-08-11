# 04 — Shared Category CRUD

**What to build:** Any Learner can create, browse, rename, and delete shared Categories, with deletion refusing to ever orphan a Vocabulary Entry.

**Blocked by:** 02 — Learner creation, selection & cookie identity.

**Status:** ready-for-agent

- [ ] `POST /api/categories` creates a shared Category; `GET /api/categories` lists Categories, sortable by creation date or alphabetically.
- [ ] `GET /api/categories/:id` reads a single Category; `PATCH /api/categories/:id` renames it.
- [ ] Category names are compared case-insensitively after trimming whitespace for uniqueness; duplicates return `409`.
- [ ] `DELETE /api/categories/:id` is rejected with `409` when it would remove the final Category Membership of any Vocabulary Entry; deletion never cascade-deletes Vocabulary Entries.
- [ ] Client screen lists Categories (sorted per the above) and supports create/rename/delete with clear messaging on rejected deletion.
- [ ] HTTP integration tests cover: creation, duplicate-name conflict, listing/sort order, rename, safe deletion, unsafe-deletion rejection.
