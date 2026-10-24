## Why

"删除进垃圾箱"这一半已经成立（`notes.deleted_at` 软删除、回收站列表、恢复），但**另一半缺失**：库里没有任何代码能真正删掉一条笔记。`grep -rn "DELETE FROM notes"` 在 Rust 侧零命中——软删除是终点，回收站只进不出。后果是：

- 用户以为"删掉了"，实际上正文一直留在库里（隐私直觉与真实状态不一致）；
- 库文件只增不减，`file_size_bytes` 单调上涨，`list_deleted` 也永不清空；
- 导入批次撤销也走软删除（`delete_import_batch`），一次误导入 5,000 条后会永久滞留在回收站。

## What Changes

- 新增**永久删除单条笔记**：仅对已软删除的笔记可用；不可恢复。
- 新增**清空垃圾箱**：一次性永久删除回收站里的全部笔记。
- 两者都必须先确认，计数与"不可恢复"写在确认文案里；不得提供任何撤销。
- 回收站视图补齐：删除时间、条数、单条永久删除、清空按钮、空态。
- 明确契约：**永久删除只由用户显式触发**（单条或清空）；删除笔记、导入批次撤销、任何自动流程都只能进回收站。永久删除后 `source_path` 被释放，因此同一文件可被再次导入（与"软删除仍跳过"形成对照）。

## Capabilities

### New Capabilities

（无）

### Modified Capabilities

- `notes-core`: 现有 `收藏与软删除恢复` 只声明"删除是软删除、可恢复"，没有声明软删除的笔记如何离开系统。新增两条 requirement——`永久删除`与`回收站视图`——把出口写成契约；既有软删除/恢复语义不变，不重写。

## Impact

- 后端：`repositories/note_repo.rs` 新增 `purge(id)` 与 `purge_all()`；`commands/notes.rs` 新增 `purge_note` / `empty_trash`；`lib.rs` 注册。无 schema 变更（不需要墓碑列）。
- DTO：`NoteSummary` 增加 `deleted_at: Option<i64>`（加性字段，普通列表为 `null`），让回收站能显示真实删除时间；TS 侧 `NoteSummary` 同步加 `deletedAt: number | null`。
- 前端：`features/notes/TrashList.vue` 增加删除时间、单条永久删除、清空垃圾箱与空态；`stores/notes.ts` 增加 `purgeSelected()` / `emptyTrash()`；`services/commands.ts` 增加两个调用。
- 数据：`note_tags` 靠既有 `ON DELETE CASCADE` 清理；`notes_fts` 必须显式删除（不是外键关系）。
- 与导入的交互：永久删除一条导入的笔记后，`source_path` 唯一索引不再被占用，重导同一目录会重新导入它（`source_path_exists` 只在笔记存在时跳过）。
- 测试：`tests/import.rs`、`tests/database.rs` 需要补"永久删除后重导可再次导入"与"清空不影响未删除笔记"。
