-- v4: import provenance. Both columns are nullable and only set by the
-- Markdown importer; hand-written notes keep NULL. The unique index is partial
-- so the "one row per source file" guarantee does not apply to real notes.
ALTER TABLE notes ADD COLUMN source_path TEXT;
ALTER TABLE notes ADD COLUMN content_hash TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_notes_source_path
  ON notes(source_path) WHERE source_path IS NOT NULL;
