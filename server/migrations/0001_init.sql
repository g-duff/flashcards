-- Initial migration. Establishes the migration mechanism; domain tables
-- (learners, categories, vocabulary_entries, etc.) are added by later
-- migrations as each area of the domain is implemented.
PRAGMA foreign_keys = ON;
