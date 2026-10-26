-- v5: 速记（Quick Tab）的独立表。
--
-- 刻意与笔记体系完全隔离：不引用 notes、不参与 notes_fts、没有标签关联、
-- 没有 deleted_at（删除即永久删除）。隔离在 schema 层成立，而不是靠每个
-- 查询记得加过滤条件（design D1）。
CREATE TABLE IF NOT EXISTS quick_tabs (
  id TEXT PRIMARY KEY NOT NULL,
  title TEXT NOT NULL DEFAULT '',
  color TEXT NOT NULL,
  content TEXT NOT NULL DEFAULT '',
  sort_order INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_quick_tabs_sort ON quick_tabs(sort_order, created_at);
