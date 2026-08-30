# Flashcards — v1 design

The plan the v1 build works from. Domain vocabulary is in
[`../CONTEXT.md`](../CONTEXT.md); the one recorded decision is
[ADR-0001](adr/0001-opaque-schedule-state.md).

## Goal

A single-user flashcards app for learning a language: capture vocabulary
as translation pairs, practise each pair in both directions on a Leitner
schedule. Ship a working practice loop first; add depth later. The
existing scaffold stays — React SPA, Rust/axum backend, container build,
one-container local run.

## Scope

**In v1**

- Create, list, delete Terms; edit a Term's `notes`.
- Bulk import Terms from a delimited file (parsed in the browser).
- Every Term yields two Cards — recognition and production.
- Practice: fetch due Cards, flip, self-grade pass/fail, server
  reschedules via the Leitner `Scheduler`.
- Every graded attempt is written to an append-only `review` log.
- SQLite persistence (replaces the in-memory store).

**Deferred — not in v1**

- Typed-answer or multiple-choice practice modes.
- Any Scheduler other than Leitner (the seam is built; the impl is not).
- Decks, topics, or tags — one flat deck.
- Multiple users, accounts, auth.
- Editing a Term's text (identity — see Identifiers).
- A persisted practice session (resume, session history).
- A stats / progress screen (the `review` table accumulates the data).
- JSON import endpoint variants, audio, structured part-of-speech or
  gender fields.

## Data model

Three tables. All ids are `TEXT`. All timestamps are ISO-8601 `TEXT`
(`chrono`). `␟` is the `\x1f` unit separator.

```
  ┌─────────────────────────────────────┐
  │ term                                │
  ├─────────────────────────────────────┤
  │ id            TEXT  PK   (uuid v5)   │◄────────────┐
  │ foreign_lang  TEXT  NOT NULL         │             │
  │ foreign_text  TEXT  NOT NULL         │             │
  │ pivot_text    TEXT  NOT NULL         │             │  term_id
  │ notes         TEXT  NULL             │             │  FK, ON DELETE CASCADE
  │ created_at    TEXT  NOT NULL         │             │
  └─────────────────────────────────────┘             │
     id = uuidv5(APP_NS,                               │
         NFC(trim(foreign_lang)) ␟                     │
         NFC(trim(foreign_text)) ␟                     │
         NFC(trim(pivot_text)))          case-kept     │
                                                       │
              1 term  ──<  exactly 2 cards             │
                                                       │
  ┌─────────────────────────────────────┐             │
  │ card                                │             │
  ├─────────────────────────────────────┤             │
  │ id             TEXT  PK   (uuid v5)  │─────────────┘
  │ term_id        TEXT  NOT NULL  FK    │◄────────────┐
  │ prompt_side    TEXT  NOT NULL        │             │
  │                CHECK IN              │             │
  │                ('foreign','pivot')   │             │  card_id
  │ due_at         TEXT  NOT NULL  INDEX │             │  FK, ON DELETE CASCADE
  │ schedule_state TEXT  NOT NULL (JSON) │             │
  │ created_at     TEXT  NOT NULL        │             │
  ├─────────────────────────────────────┤             │
  │ UNIQUE(term_id, prompt_side)         │             │
  └─────────────────────────────────────┘             │
     id = uuidv5(APP_NS, term_id ␟ prompt_side)        │
     schedule_state: Leitner writes {"box": N}         │
     see ADR-0001                                      │
                                                       │
              1 card  ──<  0..n reviews                │
                                                       │
  ┌─────────────────────────────────────┐             │
  │ review                              │             │
  ├─────────────────────────────────────┤             │
  │ id          TEXT  PK   (uuid v7)     │             │
  │ card_id     TEXT  NOT NULL  FK       │─────────────┘
  │ rating      TEXT  NOT NULL           │
  │             CHECK IN ('pass','fail') │
  │ reviewed_at TEXT  NOT NULL  INDEX    │
  └─────────────────────────────────────┘
     id = uuidv7()  — time-ordered append-only log
```

### Table notes

- **term.** Text is immutable: `(foreign_lang, foreign_text, pivot_text)`
  is the identity and feeds the id hash. A typo is fixed by deleting the
  Term and re-importing it. `notes` is the only editable column.
- **card.** Created in pairs when a Term is inserted, one per
  `prompt_side`, both starting in Leitner box 1 with `due_at = now`. The
  `schedule_state` blob is owned by the `Scheduler` (ADR-0001). Selection
  for practice is purely `due_at <= :now` ordered by `due_at`.
- **review.** Append-only. Never updated or deleted except by cascade
  when its Card's Term is deleted. `rating` is `pass` / `fail` now; the
  column is free-form-checked so a finer scale can be added without a
  migration.

### Foreign keys

`PRAGMA foreign_keys = ON` at connection open. Both FKs are
`ON DELETE CASCADE`, so `DELETE FROM term WHERE id = ?` removes the
Term's two Cards and all their Reviews.

## Identifiers

| Table | Version | Derived from |
|---|---|---|
| `term` | UUID **v5** | `uuidv5(APP_NS, NFC(trim(foreign_lang)) ␟ NFC(trim(foreign_text)) ␟ NFC(trim(pivot_text)))` |
| `card` | UUID **v5** | `uuidv5(APP_NS, term_id ␟ prompt_side)` |
| `review` | UUID **v7** | random, time-ordered — a Review is an event, not content |

- `APP_NS` is one hardcoded namespace UUID constant, defined once in
  `core.rs`.
- The canonical string normalises with Unicode **NFC** and trims outer
  whitespace. Case is **preserved** (`él` and `el` are different words).
  `notes` is **not** part of the hash.
- Because term and card ids are deterministic, **import is idempotent**:
  re-importing the same Term produces the same id and
  `INSERT ... ON CONFLICT(id) DO NOTHING` skips it. `skipped` in the
  import response is the conflict count.
- Ids stored as `TEXT` (readable in the `sqlite3` CLI), matching the
  fleet `downloads` app.

## HTTP API

Mounted under `/flashcards/api/` (nginx strips the prefix). Conventions
carried over from the current scaffold: **success is always `200`**;
every error is `{ "error": "<message>" }`; `GET /healthz` returns
`text/plain` `ok`.

| Method | Path | Request body | 200 response |
|---|---|---|---|
| `GET` | `/healthz` | — | `ok` |
| `GET` | `/openapi.yaml` | — | the spec (`application/yaml`) |
| `GET` | `/terms` | — | `[Term]` |
| `POST` | `/terms` | `NewTerm` | `Term` (also creates its two Cards) |
| `PATCH` | `/terms/{id}` | `{ "notes": string \| null }` | `Term` |
| `DELETE` | `/terms/{id}` | — | `{ "deleted": "<id>" }` |
| `POST` | `/terms/import` | `[NewTerm, ...]` | `{ "imported": N, "skipped": M }` |
| `GET` | `/cards` | — (`?due_before=<iso8601>&limit=<n>`) | `[PracticeCard]` |
| `POST` | `/cards/{id}/reviews` | `{ "rating": "pass" \| "fail" }` | `PracticeCard` (updated `due_at`, `box`) |

Errors: `400` for a malformed body or a failed validation (empty text,
bad `prompt_side`, unparseable `due_before`); `404` for an unknown
`{id}`.

### Schemas

```
NewTerm
  foreign_lang  string   non-empty; ISO 639-1 (e.g. "es")
  foreign_text  string   non-empty
  pivot_text    string   non-empty
  notes         string?  optional

Term
  id            string   uuid v5
  foreign_lang  string
  foreign_text  string
  pivot_text    string
  notes         string | null
  created_at    string   ISO-8601

PracticeCard          (a Card with its Term's text resolved for display)
  id            string   uuid v5
  term_id       string
  prompt_side   "foreign" | "pivot"
  prompt        string   the text shown        (foreign_text or pivot_text)
  answer        string   the text to recall    (the other one)
  notes         string | null
  due_at        string   ISO-8601
  box           integer  current Leitner box   (from schedule_state)

Review                (returned inside PracticeCard flow; not a standalone resource)
  rating        "pass" | "fail"
```

- `GET /cards` with no query returns every Card. `due_before` +`limit`
  is how the practice screen pulls its queue
  (`due_before = now`, `limit ≈ 20`).
- `POST /cards/{id}/reviews` is where the server owns the schedule
  transition: it appends a `review` row, runs
  `Scheduler::on_review(schedule_state, rating, now)`, writes the new
  `schedule_state` and `due_at` back to the Card, and returns the
  updated `PracticeCard`.

## Server modules

Keeps the existing functional-core / imperative-shell split.

| Module | Responsibility |
|---|---|
| `core.rs` | Pure. `APP_NS`; `canonical_name(&NewTerm) -> String`; `term_id`, `card_id` (uuid v5); `validate_new_term`; validation of an import array; the `Scheduler` trait and its `Leitner` impl: `initial_state(now) -> (schedule_state, due_at)` and `on_review(schedule_state, rating, now) -> (schedule_state, due_at)`. Unit-tested in-file. |
| `model.rs` | Wire types: `NewTerm`, `Term`, `NotesPatch`, `Card`, `PracticeCard`, `NewReview`, `ImportReport`. Plain data. |
| `store.rs` | `Db`: `rusqlite` (`bundled`) behind `Arc<Mutex<Connection>>`, every call via `tokio::task::spawn_blocking` (`Db::with`), WAL, `PRAGMA foreign_keys = ON`. Embedded migrations via `rusqlite_migration`. Queries: `insert_term` (term + 2 cards in a transaction), `list_terms`, `patch_term_notes`, `delete_term`, `import_terms`, `due_cards(due_before, limit)`, `record_review` (append review + rescheduled card update, one transaction). No `seeded()` constructor. |
| `http/` | Handlers (thin — parse, call `store`/`core`, map result), `error.rs` (`AppError` -> `{error}` + status), router. |
| `main.rs` | Wiring: read `DATABASE_PATH` and `PIVOT_LANG`, open `Db`, run migrations, build the router, bind. |

The Scheduler seam: handlers hold `&dyn Scheduler` (or a generic
parameter) resolved once in `main.rs`. Swapping Leitner for another impl
is a one-line change there plus the new `core.rs` type — no schema
change (ADR-0001).

## Client

Two screens, plus a persistent "N due" indicator.

**Vocab**
- Table of Terms: `foreign_text`, `pivot_text`, `foreign_lang`, `notes`.
- Add a Term (inline form → `POST /terms`).
- Edit `notes` inline (`PATCH /terms/{id}`). Text fields are read-only.
- Delete a Term (`DELETE /terms/{id}`), with a confirm — it takes the
  progress with it.
- **Import**: pick a file, set the delimiter (text field, default `,`),
  parse in the browser into `[NewTerm]`, show a parsed-row count and any
  parse errors with line numbers, then `POST /terms/import`. Show
  `imported` / `skipped` from the response.

**Practice**
- Reads `GET /cards?due_before=<now>&limit=20`.
- Per Card: show `prompt` → reveal `answer` → **Pass** / **Fail** →
  `POST /cards/{id}/reviews` → next. A failed Card is shuffled back into
  the remaining queue for the same run.
- End-of-run summary: counts passed / failed / seen.

Existing client conventions hold: `Optional<T>` / `Result<T, E>` from
`types/effects`, discriminated-union view state, pure helpers
(`client/CODING_STANDARDS.md`).

## Configuration

| Env var | Default | Meaning |
|---|---|---|
| `BIND_ADDR` | `127.0.0.1:8081` | unchanged |
| `DATABASE_PATH` | `./data/flashcards.db` | SQLite file; parent dir created on open. Under systemd, the service state dir. |
| `PIVOT_LANG` | `en` | ISO 639-1 code of the pivot language. App-wide. |

## Getting vocabulary in

There is **no seed**. A fresh database is empty; first run shows an empty
Vocab table.

Everything is loaded through the **Import** control — the same path for
first-time setup, for adding a batch later, and for test fixtures. The
file is parsed in the browser (custom delimiter → `[NewTerm]`); the
server only ever receives JSON.

- A sample file lives at `dev/sample-vocab.csv` (~15 Spanish Terms) for
  first-run setup and manual testing.
- Backend tests `POST /terms/import` a JSON fixture directly.
- Frontend tests drive the Import control with a small delimited string.

## Migrations

`rusqlite_migration` with migrations embedded in the binary, version
tracked in SQLite's `user_version`. v1 is a single migration creating the
three tables and their indexes. A real migration framework / runner is
only worth adding when a schema change needs to transform existing rows.

## Likely next steps (not commitments)

- Per-Term "drill both ways?" toggle if double review volume bites.
- A second `Scheduler` (SM-2 / FSRS), rebuilt from the `review` log.
- Decks or tags once the flat deck is unwieldy.
- A progress screen off the `review` data.
- Persisted practice sessions.
