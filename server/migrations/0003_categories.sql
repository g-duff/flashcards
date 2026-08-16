-- Shared Categories: organize Vocabulary Entries, available to every
-- Learner (grilled-spec.md sec. 2, 6).
CREATE TABLE categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    normalized_name TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
