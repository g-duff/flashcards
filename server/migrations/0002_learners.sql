-- Learner profiles: durable local profiles with unique display names
-- (grilled-spec.md sec. 6).
CREATE TABLE learners (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    normalized_name TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
