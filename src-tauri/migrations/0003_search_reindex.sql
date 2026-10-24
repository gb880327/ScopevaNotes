-- v3: the FTS projection becomes CJK-aware (one token per CJK character).
-- Existing rows carry the old format, so the table is recreated empty here and
-- rebuilt from `notes` in Rust immediately after this migration runs.
DROP TABLE IF EXISTS notes_fts;
CREATE VIRTUAL TABLE notes_fts USING fts5(
  note_id UNINDEXED,
  title,
  content,
  tags_text,
  tokenize = 'unicode61'
);
