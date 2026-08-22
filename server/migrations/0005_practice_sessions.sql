-- Direction-specific Progress, Practice Sessions and their generated
-- Questions (grilled-spec.md sec. 2, 6; ticket 07). `direction_progress`
-- rows are created lazily on first Answer Submission (ticket 08); the table
-- is introduced here because session generation reads `last_correct_at` to
-- enforce the hard retest cooldown (grilled-spec.md sec. 4).
CREATE TABLE direction_progress (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    learner_id INTEGER NOT NULL REFERENCES learners (id),
    vocabulary_entry_id INTEGER NOT NULL REFERENCES vocabulary_entries (id),
    direction TEXT NOT NULL,
    total_correct_count INTEGER NOT NULL DEFAULT 0,
    total_incorrect_count INTEGER NOT NULL DEFAULT 0,
    current_correct_streak INTEGER NOT NULL DEFAULT 0,
    last_correct_at TEXT,
    last_incorrect_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (learner_id, vocabulary_entry_id, direction)
);

-- A bounded activity for one Learner, one Category, and one Translation
-- Direction (grilled-spec.md sec. 2).
CREATE TABLE practice_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    learner_id INTEGER NOT NULL REFERENCES learners (id),
    category_id INTEGER NOT NULL REFERENCES categories (id),
    direction TEXT NOT NULL,
    status TEXT NOT NULL,
    requested_question_count INTEGER NOT NULL,
    actual_question_count INTEGER NOT NULL,
    answered_question_count INTEGER NOT NULL DEFAULT 0,
    correct_count INTEGER NOT NULL DEFAULT 0,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    last_activity_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- The immutable, per-question snapshot taken at session-creation time
-- (grilled-spec.md sec. 2). `options_snapshot` stores the full option list,
-- including `is_correct`, as JSON; the public API redacts `is_correct`
-- until an Answer Submission exists (ticket 08).
CREATE TABLE practice_questions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL REFERENCES practice_sessions (id),
    vocabulary_entry_id INTEGER NOT NULL REFERENCES vocabulary_entries (id),
    direction TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    prompt_text_snapshot TEXT NOT NULL,
    correct_text_snapshot TEXT NOT NULL,
    options_snapshot TEXT NOT NULL,
    created_at TEXT NOT NULL
);
