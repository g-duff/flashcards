# 01: SQLite-backed Terms

**What to build:** Persistent vocabulary management that replaces the in-memory `Card{front,back}` deck entirely. A learner can add a Term (foreign text, its pivot-language translation, the foreign language code, optional notes), see every Term in a table, edit a Term's notes, and delete a Term — and everything survives a server restart.

A Term's three text fields are its identity and are never editable; only `notes` changes. The Term id is a deterministic UUIDv5 derived from the normalised text, so the same Term always gets the same id (this is what makes import idempotent in ticket 05).

**Blocked by:** None (can start immediately).

**Status:** ready-for-agent

## Acceptance criteria

- [ ] `term` table created by an embedded migration (`rusqlite_migration`, version in `user_version`); columns per `docs/DESIGN.md` (`id`, `foreign_lang`, `foreign_text`, `pivot_text`, `notes` nullable, `created_at`).
- [ ] `Db` layer: `rusqlite` with the `bundled` feature, single `Arc<Mutex<Connection>>` reached via `tokio::task::spawn_blocking`, `journal_mode = WAL`, `PRAGMA foreign_keys = ON` at open. Parent directory of the DB file created on open. No `seeded()` constructor.
- [ ] `core.rs` gains, as pure unit-tested functions: `APP_NS` namespace constant, `canonical_name` (NFC + trim, `\x1f`-joined, case preserved, `notes` excluded), `term_id` (UUIDv5 over `canonical_name`), and `validate_new_term` (each text field non-empty).
- [ ] `GET /terms` → `[Term]`; `POST /terms` (`NewTerm`) → `Term`; `PATCH /terms/{id}` (`{notes}`) → `Term`; `DELETE /terms/{id}` → `{deleted: id}`. Success `200`, errors `{error: msg}` with `400` (bad body / validation) or `404` (unknown id) — matching the existing scaffold convention.
- [ ] The old card model, store, handlers, routes, and their OpenAPI paths are removed. `openapi.yaml` rewritten to describe `/terms` and the new schemas; `GET /openapi.yaml` still serves it.
- [ ] Vocab screen: table of Terms; inline add form; inline notes edit; delete with a confirm. Old list/add-card UI removed. Client conventions from `client/CODING_STANDARDS.md` (`Optional`/`Result`, discriminated-union view state) upheld.
- [ ] `DATABASE_PATH` (default `./data/flashcards.db`) and `PIVOT_LANG` (default `en`) read in `main.rs`.
- [ ] Container build and `dev/` local run updated for the SQLite C toolchain dependency; the dev DB persists in a volume across `./dev/down.sh` + `./dev/up.sh`.
- [ ] `cargo test` and the client test suite pass; `cargo clippy` clean.

## How to test it yourself

1. `./dev/up.sh`, open `http://localhost:8080/flashcards/`.
2. Add a Term — foreign text `perro`, pivot text `dog`, foreign lang `es`, notes `el perro (m)`. It appears in the table.
3. Edit its notes to `el perro`. Reload the page — the edit stuck.
4. `curl http://localhost:8080/flashcards/api/terms` — the Term is there with a UUID id.
5. `POST` the exact same Term again via curl — you get a Term back with the **same id** (no duplicate row on reload).
6. `curl -X POST .../api/terms -d '{"foreign_lang":"es","foreign_text":"","pivot_text":"x"}'` → `400 {"error": ...}`.
7. `curl -X PATCH .../api/terms/<bad-id> -d '{"notes":"x"}'` → `404`.
8. Delete the Term in the UI. `./dev/down.sh` then `./dev/up.sh` — deck is still empty (persistence + delete both held).
9. Open `http://localhost:8080/flashcards/openapi.yaml` — it describes `/terms`, no `/cards`.
