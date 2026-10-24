## Why

标签现在只有"能建"这一半：侧栏的 `＋` 可以输入名称新建标签，但**没有任何界面能重命名、改色或删除**（后端 `rename_tag` / `delete_tag` 早已实现并被测试覆盖，却没有调用方），笔记上的标签也只读——`NoteHeader.vue` 只把 `note.tags` 渲染成一排死文本，用户无法给已有笔记加标签或摘标签。结果是标签系统实际上退化成"导入时从 frontmatter 带进来的一堆不可维护的字符串"。

`notes-core` 的 `标签管理` requirement 已经把重命名/删除写成契约，但这些契约没有对应的界面，测试也只能在仓库层验证。

## What Changes

- 新增**标签管理界面**：在一个地方看到全部标签（名称、颜色、笔记数），可新增、重命名、改色、删除。删除前显示"将被摘除的笔记数"，删除只摘关联，不删笔记。
- 新增**笔记标签编辑**：笔记头部的标签区变成可编辑的 chip 行（选择已有标签 / 输入新名称即建即挂 / 点 `×` 摘除），保存走既有 `update_note` 写路径与自动保存。
- 侧栏标签行的 `＋` 保留（新建快），但指向同一个 store 动作，错误与冲突提示统一。
- 不新增能力域：标签的数据模型、唯一性、索引维护规则不变，只补界面与"给笔记加/摘标签"的写入口。

## Capabilities

### New Capabilities

（无）

### Modified Capabilities

- `notes-core`: 现有 `标签管理` requirement 只声明"支持重命名与删除"，没有声明用户如何触达、删除时如何提示、如何防止误删；`标签列表带计数` 也没有声明管理界面的展示顺序。本次补充两条 requirement——`标签管理界面`与`笔记标签编辑`——把界面行为写成契约；既有 `标签管理` / `标签列表带计数` 的语义保持不变，不重写。

## Impact

- 前端：`src/features/tags/`（TagList/TagEditor 之上新增管理视图）、`src/features/notes/NoteHeader.vue`（标签行可编辑）、`src/stores/notes.ts`（`renameTag` / `deleteTag` / 笔记标签增删动作）、`src/features/settings/SettingsDialog.vue`（管理入口 section）。
- 后端：无 schema 变更。`tag_repo::rename` / `delete` 已有；新增"删除前统计受影响笔记数"的只读查询（或复用 `list_with_counts`）。`note_repo::replace_tags` 已能在 `update` 中同步索引。
- 数据：标签被删除后笔记失去关联、FTS 重建（已有行为）；`source_path` / 导入批次标签不受影响。
- 测试：`tests/database.rs` 已有重命名/删除用例；需要补"界面动作透传到命令"的组件测试与 store 测试。
