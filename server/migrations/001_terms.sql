-- v1: the term table. One row per vocabulary pair; the three text
-- columns are the Term's identity and feed its UUIDv5 id (see core.rs),
-- so only `notes` is ever updated. `notes` is the sole nullable column.
CREATE TABLE term (
    id           TEXT PRIMARY KEY,
    foreign_lang TEXT NOT NULL,
    foreign_text TEXT NOT NULL,
    pivot_text   TEXT NOT NULL,
    notes        TEXT,
    created_at   TEXT NOT NULL
);
