CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
  note_id UNINDEXED,
  title,
  content,
  tags_text,
  tokenize = 'unicode61'
);
