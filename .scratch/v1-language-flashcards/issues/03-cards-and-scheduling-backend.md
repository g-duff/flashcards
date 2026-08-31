# 03: Cards + scheduling wired (backend)

**What to build:** The practice loop, server side, verifiable by curl and tests with no UI. Creating a Term now also creates its two Cards — one prompting with the foreign text (recognition), one prompting with the pivot text (production) — each scheduled by the `Scheduler` from ticket 02. A client can ask for the Cards that are due, get back everything needed to show a prompt and its answer, submit a pass/fail grade, and see the Card's next-due date move. Every grade is written to an append-only log.

**Blocked by:** 01 (Terms + `Db` + id helpers), 02 (`Scheduler` trait + box-model impl).

**Status:** resolved

## Progress (2026-08-31)

All acceptance criteria met. `cargo test` green (54 tests), `cargo clippy`
clean except one **pre-existing** warning unrelated to this ticket
(`core::MAX_SIDE_LEN` unused — introduced by ticket 02, left untouched).

What landed:

- **`migrations/002_cards_reviews.sql`** — `card` + `review` tables per
  the design (FKs `ON DELETE CASCADE`, `card` `UNIQUE(term_id,
  prompt_side)` + CHECK, `review` `rating` CHECK, both `*_at` columns
  indexed, `card.due_at` indexed).
- **Backfill** — migration 002's `up_with_hook` calls
  `store::backfill_cards`, creating the two Cards for every pre-existing
  Term. Idempotent (deterministic ids); covered by a store test.
- **`core.rs`** — `card_id(term_id, prompt_side)` (UUIDv5 over
  `term_id ␟ prompt_side`), `prompt_and_answer`, `card_seeds` (the one
  place a Term's two Cards are built — shared by `POST /terms` and, later,
  `/terms/import`). `scheduler.rs` gains `Leitner::box_of` and
  `Rating::as_str`. All unit-tested in-file.
- **`model.rs`** — `PromptSide` enum, `PracticeCard`, `NewReview`.
- **`store.rs`** — `insert_term` now writes term + two Cards in one
  transaction; new `due_cards`, `card_schedule_state`, `record_review`
  (append review + reschedule + return updated card, one transaction).
- **`http/`** — `AppState` carries `Arc<dyn Scheduler + Send + Sync>`
  (resolved once in `main.rs`, ADR-0001 seam). `GET /cards`
  (`?due_before=&limit=`, 400 on unparseable `due_before`) and
  `POST /cards/{id}/reviews` (404 unknown id, 400 bad rating).
- **`openapi.yaml`** — `/cards`, `/cards/{id}/reviews`, `PracticeCard` +
  `NewReview` schemas, cascade note on `DELETE /terms/{id}`.
- **`Cargo.toml`** — `uuid` gains the `v7` feature (review ids).

Manual curl smoke (against a real SQLite file) confirms every step of
"How to test it yourself": two Cards per Term, due filter, pass → box 2 /
due out, fail → box 1, `maybe` → 400, unknown id → 404, delete cascades.

Note: `due_at`/`due_before` are compared as ISO-8601 text (as the design
specifies). `list_cards` now normalises `due_before` to UTC before the
query, so a non-UTC bound compares by instant. A literal `+` in the query
string still needs URL-encoding (standard).

### `/code-review` — done, cleanups applied

Two-axis review run. **Spec: faithful**, no substantive gaps. **Standards:
clean** bar judgement calls. Applied from the review:

- normalise `due_before` to UTC in `list_cards` (was: compared as the raw
  client string against `+00:00` storage);
- `record_review` takes a `core::Schedule` instead of five loose
  strings + `#[allow(clippy::too_many_arguments)]`;
- one `PRACTICE_CARD_SELECT` const (was: `FROM ... JOIN ...` repeated);
- de-dup `AppError::NotFound("card not found")` in `create_review`.

Left as deliberate v1 choices (noted, not changed):

- `Scheduler::on_review` runs just before the `record_review`
  transaction, not inside it. Single-user app; no concurrent reviews of
  one Card. Moving it in means threading `Arc<dyn Scheduler>` through
  `spawn_blocking` — not worth it for v1.
- `row_to_practice_card` reads the box out of `schedule_state` via the
  pure `Leitner::box_of` (hard-coded strategy). The ticket mandates
  `box` on `PracticeCard`; ADR-0001's concern (no SQL *filtering* on the
  blob) is respected — selection is purely `due_at`. Revisit when a
  second `Scheduler` lands.
- pre-existing `core::MAX_SIDE_LEN` unused warning (ticket 02) left
  untouched.

## Acceptance criteria

- [x] Migrations add `card` and `review` tables per `docs/DESIGN.md` (`card`: `id`, `term_id` FK `ON DELETE CASCADE`, `prompt_side` CHECK `('foreign','pivot')`, `due_at` indexed, `schedule_state`, `created_at`, `UNIQUE(term_id, prompt_side)`; `review`: `id` UUIDv7, `card_id` FK `ON DELETE CASCADE`, `rating` CHECK `('pass','fail')`, `reviewed_at` indexed).
- [x] `card_id` = UUIDv5 over `term_id \x1f prompt_side`, added to `core.rs` (pure, unit-tested).
- [x] Inserting a Term creates its two Cards in the same transaction, each via `Scheduler::initial_state`. This holds for **both** `POST /terms` and (from ticket 05) `POST /terms/import` — they share one insert path (`core::card_seeds` + `store::insert_term`).
- [x] A migration also creates the two Cards for any Terms that already exist from ticket 01.
- [x] `GET /cards` → `[PracticeCard]` (every Card). `?due_before=<iso8601>&limit=<n>` filters to `due_at <= due_before`, ordered by `due_at`, capped at `limit`. `PracticeCard` carries `prompt` / `answer` resolved from the Term by `prompt_side`, plus `notes`, `due_at`, and the current box.
- [x] `POST /cards/{id}/reviews` (`{rating}`) in one transaction: append a `review` row, run `Scheduler::on_review`, write the new `schedule_state` + `due_at` back to the Card, return the updated `PracticeCard`. Unknown id → `404`; bad `rating` → `400`.
- [x] Deleting a Term still cascades: its two Cards and all their Reviews disappear (`PRAGMA foreign_keys = ON` verified).
- [x] `openapi.yaml` updated with `/cards`, `/cards/{id}/reviews`, and the `PracticeCard` schema. Tests cover: two Cards per Term, the due filter, a pass moving `due_at` out, a fail resetting it, and the cascade. `cargo test` green, `cargo clippy` clean (bar one pre-existing unrelated warning — see Progress).

## How to test it yourself

1. `./dev/up.sh`. Add a Term via the Vocab screen (`gato` / `cat` / `es`).
2. `curl '.../api/cards'` — two Cards for that Term: one `prompt_side:"foreign"` with `prompt:"gato"`, one `prompt_side:"pivot"` with `prompt:"cat"`. Both due now.
3. `curl '.../api/cards?due_before=2099-01-01T00:00:00Z&limit=10'` — both Cards come back.
4. `curl -X POST .../api/cards/<id>/reviews -d '{"rating":"pass"}'` — response shows a later `due_at` and box 2.
5. `curl '.../api/cards?due_before=<now>'` — the passed Card is gone from the due list; its sibling is still there.
6. `curl -X POST .../api/cards/<id>/reviews -d '{"rating":"fail"}'` on the passed Card — back to box 1, due soon again.
7. `curl -X POST .../api/cards/<id>/reviews -d '{"rating":"maybe"}'` → `400`.
8. Delete the Term in the UI, then `curl '.../api/cards'` — its Cards are gone. Restart via `./dev/down.sh` + `./dev/up.sh` — still gone, review history for it gone too.
