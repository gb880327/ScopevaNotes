## Context

See `proposal.md` — Why。现状约束：

- `notes.deleted_at` 是软删除标记；`soft_delete` / `restore` 都在事务内调 `search_index::reindex_note`（该函数对已删除笔记只做删除，不插入索引行）。
- `note_tags(note_id)` 与 `note_tags(tag_id)` 都是 `ON DELETE CASCADE`，且连接上 `PRAGMA foreign_keys = ON` 常开——删 `notes` 行会自动清关联。
- `notes_fts` 是独立的 FTS5 表，**没有**外键，必须显式删除；它的 `note_id` 列是 UNINDEXED（见 `mem:search`，按该列 DELETE 是整表扫描）。
- `NoteSummary` 目前不含 `deleted_at`；`list_deleted` 复用同一 DTO。
- 导入的幂等键是 `notes.source_path` 上的部分唯一索引（`WHERE source_path IS NOT NULL`），`source_path_exists` 只看行是否存在。

## Goals / Non-Goals

**Goals:**

- 给回收站一个真实出口：单条永久删除 + 清空，且只在用户确认后发生。
- 永久删除在数据库里是彻底的：正文、标签关联、搜索索引、来源路径占用都不再留存。
- 回收站可见信息足够判断"删的是什么、什么时候删的"。

**Non-Goals:**

- 不做保留期自动清理（没有"30 天后自动删"的定时器）——那会在用户不知情时毁数据。
- 不做墓碑列 / 审计日志：永久删除就是没有记录。
- 不做 `VACUUM`（不为了回收磁盘而重写整个库文件）。
- 不做按标签/关键词批量永久删除（清空就是全部）。

## Decisions

**D1 永久删除 = `DELETE FROM notes WHERE id = ?1 AND deleted_at IS NOT NULL`。**
理由：真删而不是加 `purged_at` 墓碑列。墓碑列会让"删除"变成第二层软删除，库只增不减的老问题原样保留，且搜索/DTO 各处又得多一个过滤条件。`AND deleted_at IS NOT NULL` 是安全阀：不接受对未删除笔记的永久删除（避免"永久删除"变成绕过回收站的后门）。影响 0 行时返回 `NotFound`。

**D2 事务内顺序：先删 `notes_fts` 行，再删 `notes` 行，最后提交。**
理由：FTS 没有外键，删完 notes 就再也找不到 note_id；先删索引保证"不存在索引残留的已删除笔记"。为了不踩 UNINDEXED 陷阱，`purge_all` 用**一条** `DELETE FROM notes_fts WHERE note_id IN (SELECT id FROM notes WHERE deleted_at IS NOT NULL)`（整表只扫一次），而不是循环调 `reindex_note`（N 次全表扫描 = O(N²)，正是导入 D14 那个坑）。

**D3 `source_path` 释放是契约，不是副作用。**
理由：导入的幂等语义是"同一文件不重复导入"，实现载体是 `source_path` 唯一索引。永久删除后该文件应当可以重新导入（用户明确表示不再要它、之后又改主意），所以设计上不阻止。反过来，软删除的笔记仍然占着 `source_path`，重导会被跳过——这条差异必须写进 spec，否则"永久删除后重导又回来了"会被当成 bug 报告。

**D4 确认用应用内内联确认，文案含条数与"不可恢复"。**
理由：与导入的「删除本批」一致，可测、无原生对话框往返；永久删除没有撤销，所以文案必须把不可恢复说清楚，而不是用"确定吗？"。

**D5 `NoteSummary` 加 `deleted_at`，不用 `updated_at` 冒充删除时间。**
理由：`soft_delete` 确实把 `updated_at` 设成删除时刻，但那是实现巧合——任何"改一条已删除笔记"的路径都会让它漂移，而 DTO 里多一个可空字段的成本只是一行。普通列表该字段为 `null`。

**D6 回收站不分页，沿用 `list_deleted(limit)` 的既有上限。**
理由：软删除的笔记数量受限于用户实际操作；现有 `TrashList` 已按 limit 取。清空按钮直接作用于全库已删除行，与列表条数无关（列表可能只显示前 N 条，清空仍清全部，文案因此用"清空垃圾箱（共 M 条）"里的 M 来自全库计数而不是列表长度）。

## Risks / Trade-offs

- **误删不可恢复**：确认文案 + 不提供任何"最近删除"二道缓冲。→ 接受：这正是用户要的语义（只有清空才真删）。
- **清空是大事务**：10,000 条已删除笔记时单个事务会长时间持写锁，期间自动保存排队。→ 任务里加基准（10,000 条）；若实测单事务 > 500 ms，改为分批提交并在中途发进度事件（与导入同构）。
- **库文件不会立刻变小**：SQLite 释放的页进入 freelist，`file_size_bytes` 不变。→ `docs/RELEASE.md` 写明"永久删除不会立刻缩小库文件"，避免误判；不做 VACUUM（会让整个库重写，风险大于收益）。
- **`purge_all` 与导入/自动保存并发**：都在同一个 `Mutex<Connection>` 后面排队，不会交错；但清空期间用户点"恢复"会排在清空之后，恢复会失败（笔记已不存在）。→ 恢复失败按既有 `NotFound` 显示为可见错误即可，不额外加锁。

## Open Questions

（无。）
