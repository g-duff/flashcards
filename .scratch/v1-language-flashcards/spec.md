# v1 language-learning flashcards

The design this ticket set implements lives in the repo docs, not here:

- [`docs/DESIGN.md`](../../docs/DESIGN.md) — schema (ASCII ER diagram), HTTP API + wire schemas, server module layout, client screens, config, migrations
- [`CONTEXT.md`](../../CONTEXT.md) — domain glossary + conceptual entity diagram
- [`docs/adr/0001-opaque-schedule-state.md`](../../docs/adr/0001-opaque-schedule-state.md) — why `card.schedule_state` is an opaque blob

## Tickets

Dependency chain: `{01, 02} → 03 → {04, 05}`.

| # | Ticket | Blocked by |
|---|---|---|
| 01 | SQLite-backed Terms | — |
| 02 | Scheduler trait + box-model impl (pure core) | — |
| 03 | Cards + scheduling wired (backend) | 01, 02 |
| 04 | Practice screen | 03 |
| 05 | Bulk import | 03 |
