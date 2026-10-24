## 1. 后端：永久删除

- [x] 1.1 `note_repo::purge(conn, id)`：事务内先删 `notes_fts` 行、再 `DELETE FROM notes WHERE id = ?1 AND deleted_at IS NOT NULL`，影响 0 行返回 `AppError::NotFound`。验证：`cargo test --test database` 新增用例——软删除后 `purge` 成功，`notes` / `note_tags` / `notes_fts` 三处都查不到该 id。
- [x] 1.2 `note_repo::purge` 拒绝未删除笔记：对一条未软删除的笔记调用返回 `NotFound` 且三张表都不变。验证：同上文件断言 `deleted_at IS NULL` 时 `purge` 报错、笔记仍在。
- [x] 1.3 `note_repo::purge_all(conn) -> usize`：一条 `DELETE FROM notes_fts WHERE note_id IN (SELECT id FROM notes WHERE deleted_at IS NOT NULL)` + 一条 `DELETE FROM notes WHERE deleted_at IS NOT NULL`，同事务，返回删除条数。验证：`cargo test --test database` 用例——3 条已删除 + 2 条正常笔记，清空后回收站为空、正常笔记与它们的标签关联、正文、时间戳都不变。
- [x] 1.4 清空不误伤导入幂等：永久删除一条导入笔记后重导同一目录，该文件重新创建（`source_path_exists` 变回 false）；软删除的同名笔记仍被跳过。验证：`cargo test --test import -- --test-threads=1` 新增用例覆盖两条分支。

## 2. 命令与类型

- [x] 2.1 `commands/notes.rs` 加 `purge_note(id)` 与 `empty_trash()`，`lib.rs` 注册。验证：`cargo build` 通过，`grep` 确认两个命令在 `invoke_handler` 里。
- [x] 2.2 `NoteSummary` 增加 `deleted_at: Option<i64>`，`list_deleted` 填充真实删除时间，普通 `list` 为 `None`。验证：`cargo test --test database` 断言 `list_deleted` 返回的 `deleted_at` 非空且与 `soft_delete` 时刻一致，`list` 返回 `None`。
- [x] 2.3 `src/services/types.ts` 的 `NoteSummary` 加 `deletedAt: number | null`，`commands.ts` 加 `purgeNote(id)` / `emptyTrash()`。验证：`pnpm vue-tsc --noEmit` 通过且无 `as any`。

## 3. 前端：回收站

- [x] 3.1 `stores/notes.ts` 加 `purgeSelected()` / `purgeNote(id)` / `emptyTrash()`：成功后刷新回收站列表与笔记总数，失败保留列表并抛可见错误。验证：`stores/notes.test.ts` 新增用例——purge 后 `deleted` 列表少一条且 `refresh()` 被调用；失败时列表不变。
- [x] 3.2 `TrashList.vue` 每行显示删除时间（复用 `formatRelativeTime`）与两个操作：恢复、永久删除。验证：`TrashList.test.ts`（新建）渲染 2 条已删除笔记，断言 2 个删除时间文本与 2 组按钮。
- [x] 3.3 单条永久删除走内联确认（含标题与"不可恢复"），取消不发命令。验证：`TrashList.test.ts` 断言取消时 `purgeNote` 未被调用、确认时以正确 id 调用一次。
- [x] 3.4 头部加「清空垃圾箱（共 M 条）」按钮 + 内联确认；M 来自全库已删除计数而非列表长度；空态时不渲染该按钮。验证：`TrashList.test.ts` 三条用例——列表 1 条但总数 40 时文案显示 40；空态无清空按钮；确认后调用 `emptyTrash` 一次。
- [x] 3.5 空态文案：回收站没有笔记时显示"垃圾箱是空的"与说明，而不是空列表。验证：同上空态用例。

## 4. 验证与文档

- [x] 4.1 性能基准（`--ignored`）：`purge_all` 在 10,000 条已删除笔记下的耗时与单事务时长；把数字写进 `verification.md`。验证：`cargo test --release --test trash_bench -- --ignored --nocapture` 输出一条记录；若 > 500 ms，在 design 的 Risks 里补"改分批提交"的具体动作。
- [x] 4.2 端到端手工验证（待人工）：删除 3 条 → 回收站看到 3 条与删除时间 → 恢复 1 条 → 永久删除 1 条 → 清空 → 空态；每步核对普通列表计数与搜索结果。验证：步骤记录写入 `verification.md`。 **用户手动验证通过（2026-08-28）**：删 3 条 → 回收站看到删除时间与条数 → 恢复 1 条 → 取消一次永久删除 → 永久删除并确认 → 清空 → 空态 → 回列表核对计数。
- [x] 4.3 规格对照：2 条 requirement / 7 个 scenario 逐条映射证据写进 `verification.md`（三态）。验证：`openspec validate add-trash-purge --strict` 通过且矩阵无空洞。
- [x] 4.4 文档：`CHANGELOG.md` 增条目、`README.md` 功能清单补一行、`docs/RELEASE.md` 写明"永久删除不会立刻缩小库文件、不做 VACUUM"。验证：三处存在且与 spec 表述不冲突。
- [x] 4.5 全量回归：`pnpm vue-tsc --noEmit`、`pnpm vitest run`、`pnpm vite build`、`cargo test`（集成测试 `--test-threads=1`）。验证：四条命令全部通过。
