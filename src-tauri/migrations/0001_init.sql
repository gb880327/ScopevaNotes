PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS notes (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL DEFAULT '',
  content TEXT NOT NULL DEFAULT '',
  is_favorite INTEGER NOT NULL DEFAULT 0 CHECK (is_favorite IN (0,1)),
  deleted_at INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  last_opened_at INTEGER
);

CREATE TABLE IF NOT EXISTS tags (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL COLLATE NOCASE UNIQUE,
  color TEXT,
  created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS note_tags (
  note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
  tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (note_id, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_notes_live_updated ON notes(deleted_at, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_notes_favorite ON notes(is_favorite, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_note_tags_tag_note ON note_tags(tag_id, note_id);
