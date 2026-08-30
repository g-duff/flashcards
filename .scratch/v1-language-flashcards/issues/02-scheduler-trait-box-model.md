# 02: Scheduler trait + box-model implementation

**What to build:** The pure scheduling logic, in the functional core, with no I/O and no dependency on the database or HTTP. A `Scheduler` decides two things: the starting schedule for a brand-new Card, and how a Card's schedule changes after a pass or a fail. The v1 implementation is a box model — a passing review promotes the Card one level and pushes its next-due date further out; a failing review sends it back to the first level.

Per ADR-0001 the per-Card state is an opaque JSON blob the `Scheduler` owns; the box count and the interval per level are constants chosen here in code, not configuration and not a design decision.

**Blocked by:** None (can start immediately; lands alongside ticket 01 in `core.rs`).

**Status:** resolved

## Acceptance criteria

- [x] A `Rating` type (`pass` | `fail`) and a `Scheduler` trait in `core.rs` with:
  - `initial_state(now) -> (schedule_state, due_at)` for a new Card,
  - `on_review(schedule_state, rating, now) -> (schedule_state, due_at)`.
  - `schedule_state` is JSON (serialised opaque blob); `due_at` is a timestamp.
- [x] A box-model implementation of the trait. New Card starts at level 1, due immediately. `pass` → level + 1 (capped), next due further out per that level's interval. `fail` → level 1, due again soon. Constants (level count, per-level intervals) are named consts with a one-line rationale comment.
- [x] Unit tests in-file covering: initial state; promotion on repeated `pass` up to and including the cap; reset to level 1 on `fail` from every level; `due_at` moves forward correctly for each transition; state round-trips through its JSON form.
- [x] No use of wall-clock, filesystem, or network — `now` is always a parameter. `cargo test` green, `cargo clippy` clean.

## How to test it yourself

This ticket has no running surface — it is exercised through ticket 03. To verify it in isolation:

1. `cd server && cargo test` — the scheduler module's tests run and pass.
2. Read the test names: there is a case for the cap (a `pass` at the top level does not overflow), a case for `fail` from the top level dropping to level 1, and a case asserting the concrete `due_at` gap for at least one promotion.
3. Skim the implementation: the box count and intervals are named constants with a comment, not literals scattered through the logic.
