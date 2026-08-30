# 03: Cards + scheduling wired (backend)

**What to build:** The practice loop, server side, verifiable by curl and tests with no UI. Creating a Term now also creates its two Cards — one prompting with the foreign text (recognition), one prompting with the pivot text (production) — each scheduled by the `Scheduler` from ticket 02. A client can ask for the Cards that are due, get back everything needed to show a prompt and its answer, submit a pass/fail grade, and see the Card's next-due date move. Every grade is written to an append-only log.

**Blocked by:** 01 (Terms + `Db` + id helpers), 02 (`Scheduler` trait + box-model impl).

**Status:** ready-for-agent

## Acceptance criteria

- [ ] Migrations add `card` and `review` tables per `docs/DESIGN.md` (`card`: `id`, `term_id` FK `ON DELETE CASCADE`, `prompt_side` CHECK `('foreign','pivot')`, `due_at` indexed, `schedule_state`, `created_at`, `UNIQUE(term_id, prompt_side)`; `review`: `id` UUIDv7, `card_id` FK `ON DELETE CASCADE`, `rating` CHECK `('pass','fail')`, `reviewed_at` indexed).
- [ ] `card_id` = UUIDv5 over `term_id \x1f prompt_side`, added to `core.rs` (pure, unit-tested).
- [ ] Inserting a Term creates its two Cards in the same transaction, each via `Scheduler::initial_state`. This holds for **both** `POST /terms` and (from ticket 05) `POST /terms/import` — they share one insert path.
- [ ] A migration also creates the two Cards for any Terms that already exist from ticket 01.
- [ ] `GET /cards` → `[PracticeCard]` (every Card). `?due_before=<iso8601>&limit=<n>` filters to `due_at <= due_before`, ordered by `due_at`, capped at `limit`. `PracticeCard` carries `prompt` / `answer` resolved from the Term by `prompt_side`, plus `notes`, `due_at`, and the current box.
- [ ] `POST /cards/{id}/reviews` (`{rating}`) in one transaction: append a `review` row, run `Scheduler::on_review`, write the new `schedule_state` + `due_at` back to the Card, return the updated `PracticeCard`. Unknown id → `404`; bad `rating` → `400`.
- [ ] Deleting a Term still cascades: its two Cards and all their Reviews disappear (`PRAGMA foreign_keys = ON` verified).
- [ ] `openapi.yaml` updated with `/cards`, `/cards/{id}/reviews`, and the `PracticeCard` schema. Tests cover: two Cards per Term, the due filter, a pass moving `due_at` out, a fail resetting it, and the cascade. `cargo test` green, `cargo clippy` clean.

## How to test it yourself

1. `./dev/up.sh`. Add a Term via the Vocab screen (`gato` / `cat` / `es`).
2. `curl '.../api/cards'` — two Cards for that Term: one `prompt_side:"foreign"` with `prompt:"gato"`, one `prompt_side:"pivot"` with `prompt:"cat"`. Both due now.
3. `curl '.../api/cards?due_before=2099-01-01T00:00:00Z&limit=10'` — both Cards come back.
4. `curl -X POST .../api/cards/<id>/reviews -d '{"rating":"pass"}'` — response shows a later `due_at` and box 2.
5. `curl '.../api/cards?due_before=<now>'` — the passed Card is gone from the due list; its sibling is still there.
6. `curl -X POST .../api/cards/<id>/reviews -d '{"rating":"fail"}'` on the passed Card — back to box 1, due soon again.
7. `curl -X POST .../api/cards/<id>/reviews -d '{"rating":"maybe"}'` → `400`.
8. Delete the Term in the UI, then `curl '.../api/cards'` — its Cards are gone. Restart via `./dev/down.sh` + `./dev/up.sh` — still gone, review history for it gone too.
