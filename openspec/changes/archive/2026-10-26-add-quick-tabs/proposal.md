## Why

主窗口是一个长期知识管理界面：列表、搜索、标签、回收站、不可恢复删除，全部围绕"值得留存的笔记"。但日常还有另一类内容——还没成形的一行命令、一个临时待办、一段贴过来的报错。它们通常活不过几小时，放进笔记列表会污染搜索与标签；不记录又得切到别的应用。v1 当初显式否掉了这块（设计决策 D-006「无独立 Snippet / Quick Panel / 第三栏」，基线 `notes-core` 写着「MUST NOT 引入独立的代码片段实体」），现在要补上，并且明确它是一块**平行沙盒**：从菜单栏左键一次呼出、纯文本、无解析、无出口。

## What Changes

- 新增**速记（Quick Tab）**：菜单栏左键呼出的独立面板窗口，纯文本多行编辑，多个彩色圆点 Tab 切换，自动保存。
- 新增独立数据表 `quick_tabs`（migration `0005`），与 `notes` / `notes_fts` / `tags` 完全隔离。
- **BREAKING**：菜单栏左键的语义从"显示/隐藏主窗口"改为"显示/隐藏速记面板"；主窗口改由菜单项「显示主窗口」与全局快捷键 `CommandOrControl+Shift+D` 唤出（快捷键语义不变）。
- 速记 Tab 上限固定为 **12 个**，上限由 Rust 侧强制并把可见错误返回给界面。
- 速记**没有出口**：不提供"转换为笔记"，不进入搜索、标签、回收站、导入批次；删除即永久删除，需确认且不可撤销。用户要留存内容时走原生复制粘贴。
- 草稿双层保存：内存 draft → 300 ms 防抖写库；同时把 draft 快照写入 `localStorage`，用于崩溃/退出竞态后的恢复。
- 面板窗口：`label = "quick-tabs"`，无标题栏、置顶、不切 Space、失焦自动隐藏、Esc 隐藏、启动时创建但隐藏。
- 新增独立前端入口 `quick-tabs.html`，避免改动主窗口入口文件。
- 不做：Markdown 渲染、语法高亮、标签、搜索、云同步、`ORDER` 拖拽重排。

## Capabilities

### New Capabilities

- `quick-tabs`: 速记实体的数据契约（独立表、上限、时间戳）、面板窗口的显示/隐藏与聚焦语义、Tab 增删改复制、自动保存与草稿恢复、离开速记的唯一边界（不可搜索、不进回收站、无转换出口）。

### Modified Capabilities

- `notes-core`: 现有「唯一内容实体」只写了 `MUST NOT 引入独立的代码片段实体或第三栏视图`，需要补一条**限定例外**：速记是独立于 `Note` 的第二实体，与笔记体系完全隔离（不落 `notes`/`notes_fts`、不支持标签、不出现于列表/搜索/回收站、删除不可恢复、无任何自动进入笔记体系的通道）。`Note` 自身的字段、软删除、标签、回收站语义不变。
- `macos-shell`: 「单主窗口」需要从"只维护一个窗口、MUST NOT 创建新窗口实例"改为"主窗口 + 速记面板两个窗口，其余交互仍复用主窗口"；「菜单栏常驻」需要写明左键=速记面板、右键=应用菜单、菜单项「显示主窗口」；「退出确认」需要写明两个窗口都要 flush 才算"已保存"。全局快捷键、单实例、启动不显示主窗口三条不变。

## Impact

- 数据库：新增 `src-tauri/migrations/0005_quick_tabs.sql`（只建 `quick_tabs` 与索引）。现有 4 个 migration、`REINDEX_VERSIONS`、`notes`/`notes_fts`/`tags`/`note_tags` 一律不动。
- 后端：新增 `repositories/quick_tab_repo.rs`、`commands/quick_tabs.rs`、`models/quick_tab.rs`；新增窗口管理与托盘分支。现有 27 条 IPC 命令与所有 repository 函数零改动。
- 现有代码的 4 个必改点（爆炸半径）：`lib.rs` 的 `on_window_event`（现在任何窗口 `CloseRequested` 都会隐藏主窗口，两窗口下必须按 `label` 分支）、`lib.rs` 的 `invoke_handler`（纯追加新命令）、`platform/macos/tray.rs`（左键目标 + 一个菜单项文案，菜单项 ID 仍为 `toggle`）、`vite.config.ts`（加第二个 HTML 入口）。
- 前端：新增 `quick-tabs.html`、`src/quick-tabs.ts`、`src/windows/quick-tabs/*`、`stores/quickTabs.ts`；`services/commands.ts` 末尾追加速记命令段，`services/types.ts` 追加 DTO。`index.html` / `src/main.ts` / `src/App.vue` / 现有 `features/**` / `stores/notes.ts` 不动。
- 权限：新增独立 capability 文件只给面板最小权限；`capabilities/default.json` 保持字节不变。
- 测试：现有 169 个前端用例与全部 Rust 测试保持断言零修改；新增测试只落在新文件或新 `describe` 块。
- 协作纪律：实现前先开新分支（建议 `feat/quick-tabs`），全部工作在分支上完成，不合入即不污染 `main` 的既有功能。
