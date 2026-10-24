## Why

本仓库要交付一个 macOS 本地的 Markdown 笔记 + 代码片段管理器：常驻菜单栏、全局快捷键随时唤出、数据完全留在本机。此前只有一份 docs 里的实现计划，没有可验收的规格，导致「做到什么程度算完成」只能靠读计划文档推断。本 change 把该计划迁移为 OpenSpec 规格与任务，作为唯一验收基线。

## What Changes

- 建立 Tauri 2（Rust）+ Vue 3 + TypeScript + SQLite 双栏桌面应用，覆盖文档 §0 定义的产品边界：只有笔记类型，不做独立片段库、不置顶窗口、不做多窗口。
- 笔记数据层：CRUD、标签、收藏、软删除/恢复、乐观并发保存、串行自动保存队列。
- 编辑与预览：CodeMirror 6 源码编辑 + Markdown 只读预览，围栏代码块逐块复制、全文复制，禁用原始 HTML。
- 搜索：FTS5 全文检索（标题/正文/标签），与标签、收藏筛选组合，分页与过期结果丢弃。
- 笔记库：默认库路径、自定义路径打开/新建/迁移/备份，事务化 schema 迁移，拒绝降级打开。
- macOS 外壳：单主窗口显隐状态机、关闭即隐藏、菜单栏常驻、全局快捷键、单实例、可选开机启动、窗口居中与置顶。
- 外观：深浅色跟随系统、强调色、编辑器字号，偏好与笔记数据分离存储。
- 发布就绪：性能与可靠性基线、打包与签名要求、备份演练与故障提示。
- 清理：docs 下的规划文档改为指向 OpenSpec 的说明，不再作为规格来源。

## Capabilities

### New Capabilities

- `notes-core`: 笔记数据模型、创建/读取/更新、自动保存与并发控制、收藏、软删除与恢复、列表摘要与分页、标签管理。
- `markdown-editing`: 阅读/编辑双模式、CodeMirror 编辑、Markdown 预览、围栏代码块复制、全文复制与内容安全。
- `search`: 笔记全文检索、筛选组合、索引维护、查询输入转义、分页与结果时序。
- `library`: 笔记库文件位置与生命周期、打开/新建/迁移/备份、schema 迁移与版本保护、完整性检查与失败提示。
- `macos-shell`: 主窗口显隐行为、菜单栏、全局快捷键、单实例、开机启动、窗口几何与置顶。
- `appearance`: 主题模式、强调色、编辑器字号及其持久化。
- `release-readiness`: 性能与资源基线、数据安全演练、打包与签名、发布文档。

### Modified Capabilities

（无。`openspec/specs/` 目前为空。）

## Impact

- 代码：新增 `src/`（Vue 前端）、`src-tauri/`（Rust 后端）、`src-tauri/migrations/`、`tests/`。
- 新增依赖：Tauri 2 及 global-shortcut / single-instance / store / dialog / opener / clipboard-manager / autostart 插件；rusqlite（bundled + FTS5）、serde、uuid、thiserror；Vue 3、Pinia、Vite、Tailwind 4、CodeMirror 6、markdown-it、Shiki、vitest、vue-tsc。
- 平台：仅 macOS 12+；关闭按钮不退出、常驻菜单栏和全局快捷键是产品级要求，影响退出与生命周期实现。
- 数据：单文件 SQLite 库，默认位于 Application Support，可由用户改到任意路径（含云同步目录，需要额外约束）。
- 交付：需要签名/公证才能无警告分发；当前阶段只做本地开发构建。
