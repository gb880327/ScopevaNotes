## Why

v1 只有「新建一条空白笔记」一种录入方式，从别的应用迁移过来的用户没有任何入口 —— 上百条已有 Markdown 笔记只能一条条复制粘贴，实际上等于把迁移排除在外。最典型的来源是 Obsidian：纯 `.md` 文件、YAML frontmatter、`[[双链]]`、`#嵌套/标签`、文件夹当分类。本 change 给这类用户一条一次性的迁移通道，并在导入前后各给一份看得懂的报告，让他们敢删掉原目录。

## What Changes

- 新增「导入 Markdown」入口（托盘菜单 + 设置面板），弹出一个三阶段模态：选目录 → 预览统计 → 导入中进度 → 结果报告。
- 目录发现规则：递归收集 `.md` / `.markdown`，跳过隐藏目录与文件（含 `.obsidian/`、`.trash/`）、符号链接、`node_modules/`。
- 解析规则：剥离 YAML frontmatter 并从中取 `tags` 与 `created`；标题按 frontmatter `title` → 首个 `# H1` → 文件名（去扩展名）的顺序取值；`tags` 支持列表与逗号分隔，允许 `#` 前缀与嵌套 `a/b`；可选把父目录名作为标签。
- 编码：UTF-8（含 BOM）为主，非法 UTF-8 时回退 GB18030 并在报告中标注；仍失败则记为该文件失败，不写入半条笔记。
- 时间戳：文件 `mtime` → `updated_at`，`birthtime` → `created_at`，缺失回退当前时间，避免导入的笔记全部堆在列表顶部、把原有笔记挤下去。
- 幂等重试：`notes` 增加 `source_path` 与 `content_hash`（迁移 0004）；重复导入同一目录时按 `source_path` 跳过已导入的笔记，使「导到一半失败后重跑」不会产生两份。
- 批量执行：按批提交（约 200 条/批），导入期间通过 `ui://import-progress` 上报进度，可中途取消；已提交的批次保留。
- 报告：成功 / 跳过（已导入）/ 失败（含原因）/ 跳过的隐藏目录 / 命中但未迁移的本地图片引用数量，附逐文件失败清单与「删除本批」入口（整批打标签 `import-YYYY-MM-DD`）。
- **不迁移附件**：正文里的 `![](img/a.png)` 等引用原样保留，图片不会随笔记进入笔记库。报告必须显式给出未迁移的数量与「如需保留请留住原目录」的说明，不得静默。
- 明确非目标（本 change 不做）：目录监听/双向同步、附件或图片迁移、图片渲染、`[[双链]]` 解析与跳转、数学公式与 Mermaid 渲染、Dataview 等插件语法求值、导出回 Markdown。

## Capabilities

### New Capabilities

- `markdown-import`: 从目录批量导入 Markdown 笔记的完整行为 —— 入口与阶段、发现与跳过规则、解码与失败处理、frontmatter 与标题/标签提取、Obsidian 语法处置、幂等重试、进度与取消、报告字段与撤销。

### Modified Capabilities

（无。`openspec/specs/` 仍为空 —— v1 change 尚未归档，主规格还没有基线，本 change 只新增能力。导入复用 `notes-core` 既有的插入与索引路径；`source_path`／`content_hash` 是导入专用的可空列，不进入笔记的 DTO 契约，因此 `notes-core` 的 requirement 不变。）

## Impact

- 数据：迁移 `0004_import_source.sql` 给 `notes` 增加可空列 `source_path`、`content_hash` 与索引 `idx_notes_source_path`；既有行不受影响，无需回填。
- 后端：新增 `services/import_service.rs`（发现、解码、解析、分批、进度、报告）与命令 `import_scan`（预览统计）／`import_notes`（执行）；`note_repo` 增加批量插入路径，复用 `search_index::reindex_note`。
- 前端：`src/features/import/ImportDialog.vue`（三阶段模态）、`src/services/commands.ts` 新增两个命令类型、`src/stores/notes.ts` 在导入结束后刷新列表、`MainWindow.vue` 监听 `ui://import-markdown`。
- 权限：文件系统读取全部在 Rust 侧（无 `fs` 插件），目录选择复用已装好的 `tauri-plugin-dialog`。
- 文档：`docs/RELEASE.md` 与 `CHANGELOG.md` 增加导入说明与「附件不迁移」的限制；`README.md` 阶段清单更新。
- 不影响：备份仍是单文件 `VACUUM INTO`（因为不搬附件）、预览管线、搜索索引格式、macOS 外壳行为。
