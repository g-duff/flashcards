-- v2: the card and review tables (see docs/DESIGN.md § Data model).
--
-- A Card is one direction of a Term; every Term has exactly two, created
-- in the same transaction as the Term (see store::insert_term). `due_at`
-- is hoisted out and indexed because "which Cards are due" is the one
-- query that must stay fast and strategy-independent (ADR-0001);
-- `schedule_state` is an opaque JSON blob the active Scheduler owns.
--
-- A Review is one graded attempt at a Card, append-only. Both foreign
-- keys cascade: deleting a Term removes its two Cards and all their
-- Reviews.
--
-- The migration's up-hook (store::backfill_cards) also creates the two
-- Cards for every Term that already existed under ticket 01.
CREATE TABLE card (
    id             TEXT PRIMARY KEY,
    term_id        TEXT NOT NULL REFERENCES term (id) ON DELETE CASCADE,
    prompt_side    TEXT NOT NULL CHECK (prompt_side IN ('foreign', 'pivot')),
    due_at         TEXT NOT NULL,
    schedule_state TEXT NOT NULL,
    created_at     TEXT NOT NULL,
    UNIQUE (term_id, prompt_side)
);

CREATE INDEX card_due_at ON card (due_at);

CREATE TABLE review (
    id          TEXT PRIMARY KEY,
    card_id     TEXT NOT NULL REFERENCES card (id) ON DELETE CASCADE,
    rating      TEXT NOT NULL CHECK (rating IN ('pass', 'fail')),
    reviewed_at TEXT NOT NULL
);

CREATE INDEX review_reviewed_at ON review (reviewed_at);
