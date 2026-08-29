-- Vocabulary Entries: shared language-neutral translation pairs, plus their
-- many-to-many Category Memberships (grilled-spec.md sec. 2, 6).
CREATE TABLE vocabulary_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_language TEXT NOT NULL,
    source_text TEXT NOT NULL,
    target_language TEXT NOT NULL,
    target_text TEXT NOT NULL,
    normalized_identity TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE category_memberships (
    category_id INTEGER NOT NULL REFERENCES categories (id),
    vocabulary_entry_id INTEGER NOT NULL REFERENCES vocabulary_entries (id),
    created_at TEXT NOT NULL,
    PRIMARY KEY (category_id, vocabulary_entry_id)
);
