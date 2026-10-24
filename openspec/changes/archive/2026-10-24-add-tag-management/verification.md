# add-tag-management — 验收对照

对照对象：`openspec/changes/add-tag-management/specs/notes-core/spec.md`
（2 requirement / 9 scenario）。三态标注：**自动**（有可复跑测试或基准）、
**人工已验**（真实浏览器 / 真机点过）、**待人工**（必须在真机 UI 上点一遍）。

当前状态：**9/9 scenario 自动覆盖 + 0 空洞**；任务 5.1 的端到端手工验证已由用户
在真机完成并通过（见文末）。

实现说明只在必要处出现；规格文字是契约，这里只写「凭什么说它成立」。

## 代码落点

| 层 | 文件 | 内容 |
| --- | --- | --- |
| Rust | `repositories/tag_repo.rs` | 新增 `set_color`（只改颜色，不动关系、不重建检索投影） |
| Rust | `commands/tags.rs`、`lib.rs` | 新命令 `set_tag_color`，其余四个命令签名不变 |
| 前端 | `services/commands.ts`、`services/types.ts` | `api.setTagColor`（无新 DTO：返回既有 `Tag`） |
| 前端 | `stores/notes.ts` | `renameTag` / `deleteTag` / `setTagColor`；`attachTag` / `detachTag` / `createAndAttachTag`；`TagAttachError` |
| 前端 | `features/tags/TagList.vue` | 行结构 = 色点 + 名称 + 计数 + hover 动作；行内重命名、行内删除确认、8 色板 |
| 前端 | `features/tags/TagEditor.vue` | 就地输入框（预填 + 全选、回车提交、Esc / 失焦取消、错误内联） |
| 前端 | `features/notes/NoteHeader.vue` | 标签 chip 带 `×`，`＋` 选择器（已有标签 / 新名称即建即挂） |

写路径没有第二条：标签关系变更一律走 `update_note({ tagIds })`（同一事务里
`replace_tags` + 重建该笔记的检索投影），因此自动保存、`editVersion` 守卫、
冲突处理与正文完全同源（design D6）。

## Scenario 对照

### Requirement: 标签管理（侧栏就地编辑）

| # | Scenario | 状态 | 证据 |
| --- | --- | --- | --- |
| 1 | 侧栏就地重命名 | 自动 | `src/features/tags/TagList.test.ts › renames inline and closes the editor`（`renameTag('tag-1','database')`、输入框消失）；`src/stores/notes.test.ts › renames the tag in the sidebar and on the open note, then re-queries`（侧栏与笔记 chip 同步、列表重查）；`src-tauri/tests/database.rs › renaming_and_deleting_a_tag_keeps_notes_intact`（改名后检索投影：旧名 0 命中、新名 3 命中） |
| 2 | 点击行仍然只是过滤 | 自动 | `TagList.test.ts › clicking the name area only filters — it never starts editing`（`activeTagIds` 变化，且没有输入框、没有删除确认）；既有 `src/components/layout/Sidebar.test.ts › selects a tag from the sidebar and reports it as active` 仍绿（过滤手势未变） |
| 3 | 删除标签先确认再摘除 | 自动 | `TagList.test.ts › confirms inline with the affected note count and can be cancelled`（文案含「将影响 3 条笔记」、取消不调命令）+ `› deletes once on confirm, without touching notes`（只调一次、侧栏行随之消失）；`tests/database.rs › renaming_and_deleting_a_tag_keeps_notes_intact`（删除后 3 条笔记仍存在，正文与 `updated_at` 逐字相等，`tags` 为空）；计数来源 = `tag_repo::list_with_counts.noteCount`（`LEFT JOIN notes ... AND deleted_at IS NULL`，注释已写明「不得再按标签逐个查」） |
| 4 | 重名被拒绝并保留原状态 | 自动 | `TagList.test.ts › keeps the editor open and shows the conflict`（错误可见、输入框仍在）；`src/stores/notes.test.ts › keeps the sidebar untouched when a rename is rejected`（`tags` 保持 `db`、`error` 带原因）；Rust 侧唯一性由 `tests/database.rs › rejects_missing_note_and_duplicate_tag`（`WORK` vs `work` → `Conflict`）保障 |
| 5 | 空名称不改动标签 | 自动 | `TagList.test.ts › refuses a blank name without calling the command`（`renameTag` 未被调用、行内提示「标签名称不能为空」）；Rust `tag_repo::{create,rename}` 对空白归一化后返回 `AppError::Invalid` |

### Requirement: 笔记标签编辑

| # | Scenario | 状态 | 证据 |
| --- | --- | --- | --- |
| 6 | 添加已有标签 | 自动 | `src/features/notes/NoteHeader.test.ts › offers only the tags the note does not carry yet`（选择器不含已挂标签）+ `› attaches an existing tag and closes the picker`；`src/stores/notes.test.ts › attaches a tag through update_note and refreshes the counts`（`updateNote({ tagIds: ['tag-2'] })`、`selected.tags` 由服务端回包决定、`listTags` 重取）；计数 +1 的来源是 `list_with_counts`，其正确性由 `tests/database.rs › search_composes_with_filters_and_skips_deleted`（软删除不计入）与统计查询本身覆盖 |
| 7 | 输入新名称即建即挂 | 自动 | `NoteHeader.test.ts › creates and attaches a new tag name`（`createAndAttachTag('新标签')`）；store 侧 `createTag` → `attachTagObject` → `refreshTags`，即新标签立刻出现在侧栏（`TagList.test.ts › creates a tag from the header plus button`） |
| 8 | 摘除只影响本笔记 | 自动 | `NoteHeader.test.ts › detaches a tag from the open note` + `› renders one removable chip per tag plus an add button`；`src/stores/notes.test.ts › detaches a tag through update_note and refreshes the counts`（只提交本笔记的新 `tagIds`，命令粒度即「一条笔记」）；`tests/database.rs › renaming_and_deleting_a_tag_keeps_notes_intact` 反证：标签实体与其它笔记的关联不会被笔记侧操作影响（同一 `note_tags` 只按 `note_id` 改写） |
| 9 | 保存失败时可见且不丢输入 | 自动 | `NoteHeader.test.ts › keeps the chip and shows the error when the save fails`（错误文本出现、chip 仍挂着）；`src/stores/notes.test.ts › flags a tag that was created but could not be attached`（`TagAttachError.tagCreated === true`、消息含「已创建」，且新标签留在 `tags` 里可重试）；`src/stores/notes.test.ts › keeps the sidebar untouched when a rename is rejected` |

没有 scenario 落在「待人工」之外的空洞：9/9 有可复跑的自动证据。

## 基准（任务 1.2）

```
cd src-tauri && cargo test --release --test tag_bench -- --ignored --nocapture
delete_tag notes=1000 elapsed=517.23ms remaining_tags=0 live_notes=5
```

1,000 条共用同一标签时删除耗时 **517 ms**，低于 1 s 阈值，因此不改 design 的
风险条目（Risks 里那条「删除共用标签是 O(N²)」保持「已知、当前可接受」）。
成本全部来自 `delete` 对每条笔记调 `reindex_note`；`notes_fts.note_id` 是 UNINDEXED，
按 `note_id` 的删除是整表扫描——这条坑已写进 `.serena/memories/backend/search.md`。

## 布局与对比度（本轮真实浏览器量测）

临时 fixture 页（Vue 组件 + 预置 store 数据，未入库，验证后删除）在 Chromium 里
渲染同一套组件，量到：

- 重命名输入框宽 271 px，落在 283 px 行内（不溢出），挂载后已聚焦且预填当前名；Esc 关闭。
- 删除确认行文本为「将影响 12 条笔记 删除 取消」，两个按钮在 `.tags` 右边界内、可见
  （`opacity: 1`），Esc 收起。
- 标签选择器 320×72，位于 chip 行下方且完整可见（不是被横向滚动容器裁掉的一行）。
- 调色板 8 个色块，展开后行高 26 → 62 px，下方行整体下移；没有溢出容器。
- 新元素文字对比度（浅色）：计数 5.08、`标签` 分组标题 5.08、「全部标签」15.46、
  相对时间 5.54，均 ≥ 4.5:1；深色沿用既有令牌（`tokens.test.ts` 已在两种主题下守住
  文本令牌 × 表面 × 强调色组合）。

> 这一步先发现并修掉了两个真问题：调色板与选择器原来是 `position: absolute`，
> 分别被 `.scroll`（`overflow-y: auto`）和 `.meta`（`overflow-x: auto`）裁剪，
> 打开时只露出一条边。改成随行内展开的流式面板后，贴着窗口底部也不会被切掉。

## 端到端手工验证（任务 5.1，人工已验）

用户在真机上按下列顺序走了一遍，逐步核对侧栏计数、过滤结果与笔记头部，结果符合预期：

1. 侧栏 `＋` 新建 `db`；点行首色点选紫色。
2. 打开一条笔记，`＋` 挂上 `db` → 侧栏 `db` 计数 +1。
3. 同一笔记 `＋` 输入 `database` 回车 → 新标签出现并立刻挂上。
4. 侧栏把 `db` 重命名为 `sqlite` → 笔记 chip 与按 `sqlite` 的过滤同时变。
5. 从笔记 `×` 摘除 `sqlite` → 侧栏计数 -1，其它笔记的 `sqlite` 仍在。
6. 侧栏删除 `sqlite`（确认文案核对条数）→ 相关笔记仍在列表里，只是没有该标签。

第 6 步的条数应与「未删除笔记数」一致（回收站里的笔记不计入）。

这些步骤无法在本环境自动执行：macOS 无 WebDriver 驱动 WKWebView，`screencapture`
取不到窗口（黑帧），只能由人在真机点。上表的自动证据覆盖每一步的**逻辑**，5.1 覆盖
**连续性**（真实数据 + 真实点击顺序）。至此没有开放的验收项。

> 记录方式：5.1 的结论来自用户本人的手动验证（本 change 的验收主体就是用户本人，
> 见 v1 归档 design D20），没有截图或录屏留档，这是本机自用范围的既有边界。
