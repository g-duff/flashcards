# Card scheduling state is an opaque JSON blob

The `Scheduler` is a pluggable strategy: Leitner boxes now, potentially
SM-2 or FSRS later. Each algorithm needs a different per-card state shape
(Leitner: a box number; SM-2: ease factor, interval, repetition count;
FSRS: stability, difficulty). Rather than give the `card` table a column
per algorithm and migrate every time the strategy changes, the row
stores:

- `due_at TEXT` — hoisted out and indexed, because "which cards are due"
  is the one query that must stay fast and must not depend on the
  strategy;
- `schedule_state TEXT` — a JSON blob the active `Scheduler` serialises
  and deserialises itself. The database never looks inside it. Leitner
  writes `{"box": 3}`.

## Considered options

- **A column per algorithm** (`box`, `ease_factor`, `interval_days`, …),
  nullable, only some populated. Rejected: every new strategy is a schema
  migration, and the table fills with columns that are meaningless for
  whichever strategy is active.
- **Typed columns for Leitner now, migrate on first change.** Rejected:
  it defeats the point of building the strategy seam up front — the first
  strategy swap would still be a migration plus a backfill.

## Consequences

- Swapping or tuning the `Scheduler` is a code change with no schema
  migration, as long as `due_at` stays meaningful.
- `schedule_state` is not queryable in SQL. That is acceptable: no
  feature needs to filter or aggregate on it — selection is entirely via
  `due_at`.
- A `Scheduler` change that cannot read the previous strategy's blob must
  rebuild each card's state. The append-only `review` table exists for
  exactly this: replay a card's Reviews through the new strategy.
