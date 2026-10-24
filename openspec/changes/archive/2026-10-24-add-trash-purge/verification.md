# add-trash-purge — 验收对照

对照对象：`openspec/changes/add-trash-purge/specs/notes-core/spec.md`
（2 requirement / 7 scenario）。三态标注：**自动**（有可复跑测试或基准）、
**人工已验**（真机点过）、**待人工**（必须在真机 UI 上点一遍）。

当前状态：**7/7 scenario 自动覆盖 + 0 空洞**；任务 4.2 的端到端手工验证已由用户在
真机完成并通过（见文末）。

## 代码落点

| 层 | 文件 | 内容 |
| --- | --- | --- |
| Rust | `repositories/note_repo.rs` | `purge`（先删 `notes_fts` 再删 `notes`，0 行 → `NotFound`）、`purge_all`、`count_deleted`；`list_deleted` 填充真实 `deleted_at` |
| Rust | `models/note.rs` | `NoteSummary.deleted_at: Option<i64>`（普通列表恒为 `None`） |
| Rust | `commands/notes.rs`、`lib.rs` | `purge_note` / `empty_trash` / `count_deleted_notes` 三个新命令 |
| 前端 | `services/types.ts`、`commands.ts` | `NoteSummary.deletedAt`、`purgeNote` / `emptyTrash` / `countDeletedNotes` |
| 前端 | `stores/notes.ts` | `deletedCount` + `purgeNote` / `emptyTrash`（先执行命令再重读回收站、列表与标签计数；失败保留列表并抛出） |
| 前端 | `features/notes/TrashList.vue` | 删除时间、每行「恢复 / 永久删除」、行内确认、清空按钮（含全库条数）、空态 |

没有新表、没有新迁移：永久删除只是把既有行彻底删掉（design D1）。

## Scenario 对照

### Requirement: 永久删除

| # | Scenario | 状态 | 证据 |
| --- | --- | --- | --- |
| 1 | 单条永久删除 | 自动 | `src-tauri/tests/database.rs › purge_removes_the_note_body_links_and_search_row`：软删除后 `purge` 成功，`rows_for()` 断言 `notes` / `note_tags` / `notes_fts` 三处计数由 `(1,1,0)` 变 `(0,0,0)`，`list` 搜标题为空；前端 `TrashList.test.ts › confirms a permanent delete inline and sends nothing on cancel`（确认后行消失、命令只发一次）、`stores/notes.test.ts › drops the purged note and re-reads the library and the tag counts` |
| 2 | 清空垃圾箱 | 自动 | `tests/database.rs › empty_trash_keeps_live_notes_and_their_deletion_timestamps_are_real`：3 条已删除 + 2 条正常，`purge_all` 返回 3、回收站清空，两条正常笔记 `(updated_at, content)` 与删除前逐字相等、标签仍在、仍能被搜到；前端 `TrashList.test.ts › clears the whole trash once confirmed, and nothing on cancel` |
| 3 | 取消确认不删除 | 自动 | `TrashList.test.ts` 的两个取消分支：单条取消后 `purgeNote` 零调用、确认面板消失；清空取消后 `emptyTrash` 零调用。store 层没有任何"乐观移除"（`purgeNote` 先调命令再重读），所以取消路径连本地列表都不会动 |
| 4 | 永久删除后释放导入来源 | 自动 | `src-tauri/tests/import.rs › purging_an_imported_note_frees_its_source_path_for_a_reimport`：软删除期间重导 `imported=0 / skippedExisting=2`；永久删除后重导 `imported=1`，且新笔记 id 不同于旧 id、`deleted_at` 为空。规格里"软删除仍被跳过"这条分支由同一用例的前半段断言 |

### Requirement: 回收站视图

| # | Scenario | 状态 | 证据 |
| --- | --- | --- | --- |
| 5 | 回收站展示删除时间 | 自动 | `TrashList.test.ts › renders one row per deleted note with its own deletion time`（两行分别显示「1 天前」「3 天前」，两组按钮齐备）；数据侧 `tests/database.rs` 同 2 号用例断言 `list_deleted` 的 `deletedAt` 全为 `Some(>0)` 且 `list` 全为 `None`；排序由 `ORDER BY deleted_at DESC` 保证（既有实现） |
| 6 | 恢复后回到普通列表 | 自动 | `TrashList.test.ts › restores a note without asking for confirmation`（调用 `restoreNote`、`store.deleted` 清空）；"回到普通列表与搜索结果"由既有 `tests/database.rs › crud_soft_delete_restore_and_search_projection`（恢复后重新出现在搜索）与 `stores/notes.ts › restoreNote` 里的 `refresh()` + `refreshTags()` 覆盖 |
| 7 | 空回收站显示空态 | 自动 | `TrashList.test.ts › shows the empty state instead of an empty list`（无 `.item`、有「垃圾箱是空的」、清空按钮不渲染）；清空后的同一断言也在 `clears the whole trash once confirmed…` 里重复一次。spec 要求「不显示清空按钮为可用状态」——实现是整颗按钮不出现，比禁用更强 |

## 基准（任务 4.1）

```
cd src-tauri && cargo test --release --test trash_bench -- --ignored --nocapture
purge_all deleted=1000 removed=1000 elapsed=10.44ms notes_left=0 fts_left=0
purge_all deleted=10000 removed=10000 elapsed=102.71ms notes_left=0 fts_left=0
```

10,000 条 103 ms，远低于 design 里定的 500 ms 线，所以**不改成批提交**，
design 的 Risks 保留原样（"若 > 500 ms 才分批"）。耗时基本是两次整表扫描
（`notes_fts` 一次、`notes` 一次），没有 per-note `reindex_note`。

单条删除没有单独基准：它是两次主键/一次整表 DELETE，成本低于 1 ms，且不随库增长。

## 测试基础设施的一处修正

`tests/database.rs` 的 `temp_db()` 原来只用 `SystemTime` 纳秒做文件名。macOS 的时钟
只到微秒，两个并行测试可能拿到同一个值，于是共用同一个库文件并撞上
`UNIQUE constraint failed: schema_migrations.version`（本轮新增用例后真实发生过）。
现在文件名里再拼一个进程内自增序号；并行跑整个文件不再需要 `--test-threads=1`。
（集成测试整体仍按项目约定用 `--test-threads=1` 运行。）

## 端到端手工验证（任务 4.2，人工已验）

用户在真机上按下列顺序走了一遍，逐步核对回收站条数、删除时间、普通列表计数与搜索，
结果符合预期：

1. 普通列表里删除 3 条笔记 → 点「回收站」，看到 3 条，条数为 3，每条都有删除时间。
2. 其中一条点「恢复」→ 它从回收站消失、回到普通列表，条数变 2；在搜索里能找到它的正文。
3. 另一条点「永久删除」→ 行内出现「永久删除后不可恢复」，点取消，什么也没变；
   再点一次并确认 → 该行消失，条数变 1，用它的关键词搜索不到。
4. 点「清空垃圾箱（共 1 条）」→ 确认 → 回收站显示「垃圾箱是空的」，清空按钮消失。
5. 回到普通列表，确认未删除笔记的数量、内容与时间戳都没变。

第 3、4 步核对的是"确认前后"的差别（取消不发请求）；第 5 步核对的是清空没有误伤。
这些无法在当前环境自动执行：macOS 无 WebDriver 驱动 WKWebView，`screencapture`
取不到窗口（黑帧）。上表的自动证据覆盖每一步的**逻辑**，4.2 覆盖**连续性**
（真实数据 + 真实点击顺序）。至此没有开放的验收项。

> 记录方式：结论来自用户本人的手动验证（本 change 的验收主体是用户本人，见 v1
> 归档 design D20），没有截图或录屏留档，这是本机自用范围的既有边界。

## 附带修掉的布局问题（侧栏标签区高度）

真机验收时发现：在「笔记」和「回收站」之间切换，左下角的标签区会上下跳。根因是
`NoteList .list` 有 `flex: 1` 而 `TrashList .trash` 没有——回收站为空时它的高度只有
一行提示，于是标签区被顶到中间；另外标签区本身是 `max-height: 38%` 的内容高度，
标签少的时候高度也不固定。

修法：`.trash` 补上 `flex: 1; min-height: 0`（和列表一样填满可用空间），标签区改成
`.tags { flex: 0 0 auto; height: 38%; min-height: 104px; max-height: 320px }`，内部
`.scroll` 用 `flex: 1; min-height: 0` 自己滚。在 Chromium 里用临时 fixture 量过
（未入库，用完删除）：

- 三种状态（笔记列表 / 空回收站 / 回收站 3 条）在同一视口下 `.tags` 的位置与高度
  **完全一致**：1680×984 → `y=664 h=320`；1400×900 → `y=580 h=320`；760×480 →
  `y=298 h=182`。上方面板三种状态下同为 `h=552 / 468 / 186`。
- 30 个标签时面板仍是 320px，内部 `.scroll` 288px 并出现滚动（滚到底后首行移出视野），
  即「固定高度 + 内部滚动」成立；侧栏没有横向溢出（`scrollWidth - clientWidth = 0`）。

另有一项**不可撤回**的提醒：永久删除在本机没有第二份副本，回收站之外没有"最近删除"。
这正是设计选择（用户明确要求不做撤回），测试也覆盖了"确认才删"，但真机上第一次
使用时值得先删一条废稿试试手感。
