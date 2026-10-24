## Why

标签现在只有"能建"这一半：侧栏的 `＋` 可以输入名称新建标签，但**没有任何界面能重命名、改色或删除**（后端 `rename_tag` / `delete_tag` 早已实现并被测试覆盖，却没有调用方），笔记上的标签也只读——`NoteHeader.vue` 只把 `note.tags` 渲染成一排死文本，用户无法给已有笔记加标签或摘标签。结果是标签系统实际上退化成"导入时从 frontmatter 带进来的一堆不可维护的字符串"。

`notes-core` 的 `标签管理` requirement 已经把重命名/删除写成契约，但这些契约没有对应的界面，测试也只能在仓库层验证。

## What Changes

- **侧栏标签行变成可编辑的**：在现有标签列表（已经是"全部标签 + 颜色点 + 可见笔记数"）里就地重命名、改色、删除。点行其余区域仍然是"按该标签过滤"，编辑入口是行内按钮，不跳转、不开新窗口。删除前在这一行内确认，并显示"将被摘除的笔记数"。
- 新增**笔记标签编辑**：笔记头部的标签区变成可编辑的 chip 行（选择已有标签 / 输入新名称即建即挂 / 点 `×` 摘除），保存走既有 `update_note` 写路径与自动保存。
- 默认仍然不新增能力域：标签的数据模型、唯一性、索引维护规则不变，只补界面与"给笔记加/摘标签"的写入口。

关于"标签与内容是否分开管理"：**数据层早就分开了**。`tags(id, name COLLATE NOCASE UNIQUE, color, created_at)` 是独立实体表，`note_tags(note_id, tag_id)` 是多对多关系表（`PRIMARY KEY (note_id, tag_id)`，两侧 `ON DELETE CASCADE`），笔记正文里不存任何标签。所以不需要为"分开管理"做数据迁移；本次要做的只是让界面也遵循这个分层——**侧栏管标签实体，笔记头部管笔记与标签的关系**。

## Capabilities

### New Capabilities

（无）

### Modified Capabilities

- `notes-core`: 现有 `标签管理` requirement 只声明"支持重命名与删除"，没有声明用户如何触达、删除时如何提示、如何防止误删。本次补充两条 requirement——`标签管理（侧栏就地编辑）`与`笔记标签编辑`——把界面行为写成契约；既有 `标签管理` / `标签列表带计数` 的语义保持不变，不重写。

## Impact

- 前端：`src/features/tags/TagList.vue`（行内重命名 / 改色 / 删除 + 行内确认）、`src/features/tags/TagEditor.vue`（复用为"新建"与"重命名"的输入框）、`src/features/notes/NoteHeader.vue`（标签行可编辑）、`src/stores/notes.ts`（`renameTag` / `deleteTag` / 笔记标签增删动作）。设置面板**不**新增 section。
- 后端：无 schema 变更。`tag_repo::rename` / `delete` 已有；新增"删除前统计受影响笔记数"的只读查询（或复用 `list_with_counts`）。`note_repo::replace_tags` 已能在 `update` 中同步索引。
- 数据：标签被删除后笔记失去关联、FTS 重建（已有行为）；`source_path` / 导入批次标签不受影响。
- 测试：`tests/database.rs` 已有重命名/删除用例；需要补"界面动作透传到命令"的组件测试与 store 测试。
