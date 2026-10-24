# macOS Notes Manager V1 — 可执行开发实施方案

> **已迁移**：本文件是历史输入。规格与任务以 `openspec/changes/v1-notes-manager/` 为准，
> 新需求请开新的 OpenSpec change，不要直接修改本文件。

> 版本：1.0 · 2026-10-24  
> 状态：产品范围和 UI 信息架构已确认；可直接作为 Codex / OpenCode / Pi 的实施输入。  
> 目标：macOS 本地优先的 Markdown 笔记和代码块管理器，支持菜单栏常驻、快捷键控制、两栏 UI、SQLite 自定义路径及一键复制代码。

## 0. 执行原则

- **只实现一个内容类型 `Note`**，其正文是 Markdown；用围栏代码块表达所有 Snippet，不维护独立 Snippet 类型或编辑器。
- **只有一个主窗口**：左侧搜索/笔记列表/标签；右侧阅读/编辑/代码复制。全局快捷键直接显示/隐藏主窗口；不实现 Quick Panel。
- 不实现账号、云同步、后端 API、富文本块编辑、AI、插件、自动展开或跨应用自动粘贴。
- Vue 3 负责 UI，Rust 负责 SQLite、文件、窗口和系统集成；前端不得直接执行任意 SQL。
- **先实现可运行的最小垂直切片，再逐阶段扩大功能**。每个里程碑有测试、验收及可回退的 Git 提交。
- 依赖版本按实现时的稳定版锁定 `pnpm-lock.yaml`、`Cargo.lock`，不要机械固定未经测试的版本。

## 1. V1 产品范围及验收

| 模块 | 必须交付 | 不交付 |
| --- | --- | --- |
| 内容 | Markdown 新建、编辑、删除/恢复、收藏、自动保存、代码块复制 | 独立 Snippet 数据模型 |
| 左栏 | 搜索框、笔记列表、标签列表、标签过滤、可调栏宽 | 三栏布局 |
| 右栏 | 标题、标签、Markdown 阅读/源码编辑、语法高亮与复制 | 专有富文本 JSON |
| 窗口 | 单主窗口、全局快捷键显示/隐藏、显示时前置/置顶/居中、调节/记忆尺寸 | 单独搜索浮窗 |
| macOS | 菜单栏图标和菜单、关闭窗口不退出、单实例、可选登录启动 | 后台云服务 |
| 数据 | SQLite `.db` 文件路径可自定义、打开/创建/迁移/备份/恢复、FTS5 | 应用内同步、跨设备冲突合并 |
| 外观 | 浅色、深色、跟随系统、强调色、编辑器字体大小 | 用户自编完整 CSS 主题 |

建议默认值：窗口 1100 × 720 逻辑像素；最小 760 × 480；左栏 300 px、可拖拽（约 220–420 px，并受窗口总宽度约束）；快捷键 `Command+Shift+Space` 可修改。实际菜单栏图标用 macOS Template Icon，不能直接用彩色 Web icon。

### 关键操作契约

1. 应用启动后注册菜单栏图标和快捷键；默认主窗口隐藏。第一次运行无笔记库时，打开必要的数据库位置引导窗口（此时允许显示主窗口）。
2. 全局快捷键切换主窗口：隐藏时在当前活动/鼠标所在显示器**可用工作区**中居中、恢复原窗口大小、设为前置且获取焦点；可见时隐藏；不关闭进程。
3. 窗口左上角关闭按钮执行隐藏；菜单栏“退出”才结束进程。关闭前需 flush 编辑器待保存内容。
4. 显示期间开启 always-on-top（用户既然明确要求置顶，先按严格置顶实现）；在 macOS 原生全屏 Space、Stage Manager、多屏情况下做专项验证。置顶不等于穿透所有系统层级。
5. 左栏筛选结果；单击笔记在右侧打开。右侧默认阅读模式；点击“编辑”切换 CodeMirror Markdown 源码编辑。
6. 阅读模式下每个 fenced code block 右上角显示“复制代码”；仅复制代码原文，不复制围栏和语言标记。另有“复制全文 Markdown”操作。
7. 笔记多标签，点击标签过滤笔记；关键词搜索包含标题、正文、标签。搜索中 Enter 默认打开选中笔记，代码块复制始终由明确按钮触发。
8. 改变数据库位置必须完整迁移或打开现有数据库，不能在失败时悄悄新建一个空库。

## 2. 技术选型与最小依赖

| 领域 | 选型 | 说明 |
| --- | --- | --- |
| 桌面 | Tauri 2 | 主窗口、菜单栏、原生菜单、打包 |
| 前端 | Vue 3 + TypeScript + Vite | 单页应用；不需要 SSR |
| UI | Tailwind CSS 4 + Reka UI | 基础组件和可访问性；无需重型组件库 |
| 状态 | Pinia | UI 状态、当前选中、编辑草稿 |
| 编辑器 | CodeMirror 6 + `@codemirror/lang-markdown` | 唯一编辑器，代码围栏语言支持可按需加载 |
| 预览 | markdown-it + Shiki | 安全渲染 Markdown、按需语言高亮；代码复制自行挂载 |
| 原生 | Rust + Tauri Plugins + 必要的 AppKit 桥接 | 不写独立常驻服务 |
| 数据 | SQLite + `rusqlite` | Rust 独占数据访问；事务、迁移、备份 |
| 搜索 | SQLite FTS5 | `unicode61` 基础索引；中文子串可评估 `trigram`，短词回退 |
| 测试 | Vitest、cargo test、Playwright（Web UI）+ macOS 实机手动/E2E | 原生窗口无法只靠浏览器测试 |

Tauri 插件按需增加：`global-shortcut`、`single-instance`、`store`、`dialog`、`autostart`、`clipboard-manager`。菜单栏可直接使用 Tauri tray/menu API。应用对能力权限采取最小授权，优先 Rust 处理系统调用。请在编译时核对 Tauri 最新 API 与 Rust MSRV，尤其全局快捷键插件。

## 3. 项目结构

```text
macos-notes/
├── package.json
├── pnpm-lock.yaml
├── vite.config.ts
├── src/
│   ├── main.ts
│   ├── App.vue
│   ├── windows/MainWindow.vue
│   ├── components/layout/{Sidebar.vue,SidebarSplitter.vue,ContentPane.vue}
│   ├── features/notes/{NoteList.vue,NoteHeader.vue,NoteEditor.vue,NotePreview.vue,CodeBlock.vue}
│   ├── features/search/{SearchInput.vue,SearchResults.vue}
│   ├── features/tags/{TagList.vue,TagEditor.vue}
│   ├── features/settings/{SettingsDialog.vue,AppearanceSettings.vue,LibrarySettings.vue}
│   ├── stores/{notes.ts,ui.ts,settings.ts}
│   ├── services/{commands.ts,types.ts}
│   ├── composables/{useAutosave.ts,useTheme.ts,useKeyboard.ts}
│   └── styles/{tokens.css,app.css}
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   ├── migrations/{0001_init.sql,0002_fts.sql}
│   └── src/
│       ├── lib.rs
│       ├── commands/{notes.rs,tags.rs,search.rs,libraries.rs,settings.rs}
│       ├── models/{note.rs,tag.rs,error.rs}
│       ├── services/{note_service.rs,search_service.rs,library_service.rs}
│       ├── repositories/{note_repo.rs,tag_repo.rs}
│       ├── database/{connection.rs,migrations.rs,backup.rs,search_index.rs}
│       ├── platform/macos/{tray.rs,shortcuts.rs,window_manager.rs,monitor.rs}
│       └── state.rs
├── tests/
│   ├── fixtures/
│   └── e2e/
└── docs/
    ├── ARCHITECTURE.md
    ├── UI_SPEC.md
    └── TEST_PLAN.md
```

Rust 层以 `AppState` 串联数据库句柄、当前库路径、窗口控制器和可取消的搜索任务。**不要将 `rusqlite::Connection` 无约束地跨异步任务共享**；采用专用数据库线程/队列，或受控连接池与 `spawn_blocking` 及事务边界，避免 UI 阻塞和死锁。

## 4. SQLite 具体设计

### 4.1 数据表（0001_init.sql）

```sql
PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at INTEGER NOT NULL
);

CREATE TABLE notes (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL DEFAULT '',
  content TEXT NOT NULL DEFAULT '',
  is_favorite INTEGER NOT NULL DEFAULT 0 CHECK (is_favorite IN (0,1)),
  deleted_at INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  last_opened_at INTEGER
);

CREATE TABLE tags (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL COLLATE NOCASE UNIQUE,
  color TEXT,
  created_at INTEGER NOT NULL
);

CREATE TABLE note_tags (
  note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
  tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (note_id, tag_id)
);

CREATE INDEX idx_notes_live_updated ON notes(deleted_at, updated_at DESC);
CREATE INDEX idx_notes_favorite ON notes(is_favorite, updated_at DESC);
CREATE INDEX idx_note_tags_tag_note ON note_tags(tag_id, note_id);
```

约定：ID 使用 UUIDv7 或 ULID（二选一并固定）；时间字段统一 Unix 毫秒 UTC；`deleted_at` 非空代表软删除；应用层负责 `updated_at`；`notes.title` 是独立元数据，正文可以包含 `#`，但不强制把首行 `#` 当标题。标签名先做 trim / 去空白与大小写规范化；`COLLATE NOCASE` 仅保证 ASCII 范围内的大小写行为，Unicode 去重应由应用侧补充。

### 4.2 搜索索引（0002_fts.sql）

```sql
CREATE VIRTUAL TABLE notes_fts USING fts5(
  note_id UNINDEXED,
  title,
  content,
  tags_text,
  tokenize = 'unicode61'
);
```

实现 `reindex_note(note_id)`：在同一数据库写事务中删除该笔记的旧搜索投影，再插入标题、原 Markdown、拼接的标签名；创建/更新笔记、修改标签关联、标签改名、软删除及恢复时均更新。搜索前必须安全构造 FTS 查询表达式，用户输入不可直接拼接为未经转义的 MATCH 语法。`trigram` 为增强项，先在目标 SQLite 版本中检测 FTS5 tokenizer 可用性，并以真实中文样例验收；1–2 字查询使用限定结果集的回退策略。不能在含标签筛选的查询中逐条 N+1 查询标签。

查询列表只返回 `id,title,excerpt,updated_at,is_favorite,tags`，不拉取全部正文；正文仅在选中时加载。第一版每页 50 条，超过时增量加载。索引更新失败需回滚同一事务，不允许笔记已保存但搜索丢失。

### 4.3 自定义数据库路径、同步目录安全

应用默认库：`~/Library/Application Support/<BundleId>/notes.db`；应用配置放在单独的 `config.json` 或 Tauri Store 中，仅记录最近库路径和 UI 偏好。用户通过原生文件对话框选择：

- **打开现有数据库**：检查文件、schema 版本、权限、`PRAGMA quick_check`，未知较新 schema 则只读保护/拒绝写入并提示。
- **新建笔记库**：选择目录与文件名，防止覆写已有文件。
- **迁移当前笔记库**：flush 待保存数据、暂停写入、制作一致性快照（SQLite Backup API 或受支持的 `VACUUM INTO`），验证新库并原子更新配置；失败时保留旧库。

SQLite WAL 默认可选，但**同步文件夹不是事务型多机数据库**：关闭应用和 checkpoint 不代表同步完成，也无法避免多设备并发编辑。文档规定同一时刻仅一台设备编辑，并等待同步结束再跨设备打开。应用不接管同步、冲突合并或上传。备份默认存放在同步目录以外的本地位置。

建议实际运行时支持按库设置 `journal_mode`，并通过跨设备同步压力测试决定默认 `WAL` 还是 `DELETE`，不要承诺文件同步工具能安全同步活动 WAL 数据库。使用 WAL 时需保留 `.db-wal` / `.db-shm` 语义，禁止只复制活动 `.db` 充当备份。

## 5. Rust / Tauri IPC 契约

前端服务封装在 `src/services/commands.ts`，每个调用返回统一的 `Result<T, AppError>` 语义（在 TS 层将 `invoke` rejection 规范化）。推荐方法：

| Command | 输入 | 输出 | 备注 |
| --- | --- | --- | --- |
| `list_notes` | `{query?,tagIds?,favorite?,limit?,cursor?}` | `NoteSummaryPage` | 不包含全文 |
| `get_note` | `{id}` | `NoteDetail` | 完整 Markdown / 标签 |
| `create_note` | `{title?,content?,tagIds?}` | `NoteDetail` | 事务创建 |
| `update_note` | `{id,title?,content?,tagIds?,expectedUpdatedAt}` | `NoteDetail` | 乐观并发检查，避免覆盖较新的数据 |
| `set_note_favorite` | `{id,value}` | `NoteSummary` | 独立更新 |
| `soft_delete_note` | `{id}` | `void` | 进入回收站 |
| `restore_note` | `{id}` | `void` | 恢复及重建索引 |
| `list_tags` | `{}` | `TagWithCount[]` | 聚合查询 |
| `create_tag` / `rename_tag` / `delete_tag` | 具体参数 | `Tag` / `void` | 维护 FTS 投影 |
| `get_library_info` | `{}` | `LibraryInfo` | 当前路径、状态、版本 |
| `create_library` / `open_library` / `move_library` | 安全的已验证路径 | `LibraryInfo` | 原生 dialog 选择 |
| `backup_library` | `{destination}` | `BackupResult` | 一致性快照 |
| `get_settings` / `update_settings` | 配置 DTO | 配置 DTO | 独立于库 |
| `toggle_main_window` | `{}` | 窗口状态 | 所有触发入口共用 |

**自动保存并发约束**：在切换笔记、关闭/隐藏窗口、切换数据库、退出应用前，必须等待当前保存 Promise 完成；编辑版本号防止旧保存回包覆盖新内容。保存失败要在 UI 明显显示“未保存”，保留草稿并提供重试；不要只依赖 `beforeunload`。

## 6. macOS 窗口与菜单栏交互

### 6.1 窗口状态机

```text
STARTING -> HIDDEN
HIDDEN --toggle/menu--> SHOWING -> VISIBLE_FOCUSED
VISIBLE_FOCUSED --toggle--> HIDDEN
VISIBLE_FOCUSED --close button--> HIDDEN
VISIBLE_FOCUSED --resize--> VISIBLE_FOCUSED (debounced size persistence)
ANY --Quit--> FLUSHING -> CLOSING_DB -> EXIT
```

所有 show/hide/toggle 请求走同一个 Rust WindowManager，带串行化/去重逻辑，避免快速连按导致状态颠倒。`SHOWING` 要：读取当前显示器可用工作区 → clamp 保存尺寸 → 计算居中坐标 → 显示 → 前置/聚焦 → 保持 always-on-top。不同 macOS Space、全屏窗口、Stage Manager 在实机检测；不以非公开 API 强行覆盖系统窗口。隐藏窗口时不销毁 WebView；应用只有一个主窗口。

### 6.2 菜单栏

左键：显示/隐藏窗口。右键：菜单（若当前 Tauri/macOS 组合难以可靠实现左右键区分，退化为单击展开菜单，菜单第一项“显示/隐藏”，并在验收中说明差异）。菜单包含：显示/隐藏、创建笔记、打开笔记库、设置、开机启动开关、退出。`new note` 在窗口隐藏时先打开并聚焦；同一菜单指令不可重复创建两个窗口。关闭窗口不退出；单实例。

### 6.3 窗口布局

主窗口默认 1100×720，最小 760×480；sidebar 默认 300px，支持拖拽，在可用宽度限制内限制为 220–420px。只记录尺寸，不必记录位置；每次被快捷键显示时重新居中到当前显示器。设置项：可配置 global shortcut；必须处理被其他应用占用的情况，失败时保留上一个有效快捷键并提示。

## 7. UI 规格与组件分解

视觉基准：此前确认的浅色 macOS 双栏设计；左侧浅灰背景，右侧简洁阅读编辑区域，原生交通灯、克制阴影和细边框。提供 light / dark / system + 蓝、紫、绿、橙、石墨强调色；主题和窗口尺寸不存入笔记数据库。

### 左栏（固定搜索 + 可滚动笔记列表 + 可滚动标签）

```text
Sidebar
├── Header: 新建笔记按钮
├── SearchInput [关键词; Cmd+F/专用焦点快捷键]
├── FilterChips [全部 | 收藏 | 最近]
├── NoteList (占剩余高度; 独立滚动)
│   └── NoteListItem [标题, 摘要, 修改时间, 标签]
├── Divider / 可调整高度（可选；先固定标签区高度）
└── TagList (固定下区; 独立滚动)
    ├── 全部标签
    └── TagItem [颜色点, 名称, 条目数]
```

### 右栏

```text
ContentPane
├── NoteHeader [标题; 收藏; 标签; 编辑/完成; 复制全文; 更多]
├── NotePreview (默认)
│   └── CodeBlock [语言; 复制按钮; 代码原文]
├── NoteEditor (编辑态, CodeMirror 6)
└── Footer [更新时间; 保存状态]
```

首次空库显示引导与“新建第一条笔记”；未选中笔记显示占位；搜索无结果显示清除筛选与新建按钮；数据库不可访问显示错误和恢复入口。代码块复制不会复制 ``` 围栏，保留行末换行原则需统一并通过测试。

### 安全渲染

Markdown 渲染默认禁用原始 HTML 或使用严格 sanitizer；外链通过 Tauri opener / 安全协议白名单打开；禁止不受控脚本、`javascript:` URL 以及未授权的本地文件资源加载。Shiki 按需加载，仅当前笔记的代码块生成高亮结果。编辑时避免对每个击键全量执行昂贵的语法高亮。

## 8. 实施里程碑（按顺序执行）

### M0 — 技术 PoC / 验证高风险能力

**任务**：初始化最小 Tauri 2 + Vue 3 应用；菜单栏图标；全局快捷键；关闭隐藏；单实例；窗口置顶居中；多显示器定位；SQLite 创建、打开、自定义路径；打包运行。

**交付**：可在真实 Mac 上运行的 PoC `.app`、`docs/POC_FINDINGS.md`，列出 Tauri 原生 API 与需要 AppKit 扩展的部分。

**验收**：快捷键反复切换 100 次不出现焦点异常或重复窗口；菜单栏退出正常；不同屏幕尺寸居中；外置显示器断开后不会在屏幕外；自定义 SQLite 能成功打开。

**门禁**：M0 未通过前，不进入大规模 UI/编辑器开发。

### M1 — 数据与业务基础

**任务**：Schema migration、rusqlite 服务、Notes CRUD、标签 CRUD 与关联、软删除/恢复、列表摘要、自动保存队列、错误处理、打开/新建/迁移/备份数据库。

**交付**：稳定 Rust commands + TypeScript DTO + 数据库集成测试。

**验收**：1,000 条随机笔记 CRUD 正确；迁移失败能恢复原库；写入中关闭窗口不丢数据；标签改名与删除后关联正确。

### M2 — 按确认 UI 落地主窗口

**任务**：双栏布局、搜索框、NoteList、TagList、可拖分隔线；右栏标题/标签/阅读区；新建、收藏、回收站；空状态；窗口尺寸与左栏宽度持久化；键盘操作与基础可访问性。

**交付**：UI 可以使用真实 SQLite 数据；不得继续使用 mock 数据作为最终功能。

**验收**：主窗口缩放时不溢出；左栏两个列表独立滚动；选择笔记后右侧显示正确内容；标签过滤有效。

### M3 — Markdown / 代码体验

**任务**：CodeMirror 6 Markdown 编辑、markdown-it 预览、Shiki 高亮、代码围栏解析、每块复制按钮、复制全文、自动保存状态、编辑/阅读切换、快捷键。

**交付**：统一 Markdown 工作流。

**验收**：包含 Bash/TS/SQL/JSON 的 Markdown 显示准确；点击代码块复制只得到代码文本；连续编辑/切换笔记不覆盖内容；预览不执行危险 HTML。

### M4 — 搜索与主题

**任务**：FTS5 投影、标题/正文/标签检索；合理排序；短词/中文处理；分页；主题 Light/Dark/System、强调色和 CodeMirror/Shiki 主题联动；搜索响应取消或丢弃过期结果。

**交付**：主窗口内即时搜索、可持久化外观设置。

**验收**：1 万条样本下正常搜索端到端 P95 < 100ms（目标，需实测）；快速输入不显示旧结果；macOS 深浅色切换时 UI/编辑器一致。

### M5 — 可靠性、性能和发布

**任务**：备份与恢复演练；同步目录异常处理；启动时路径失效；单实例压力测试；Intel/Apple Silicon 构建；签名与 notarization（如果分发）；用户安装说明；性能基准。

**交付**：正式 `.app` 包、完整 README、CHANGELOG、测试报告、故障排查手册。

**验收**：连续驻留/隐藏/打开无明显内存增长，退出时刷新全部数据；安装到目标 macOS 上可独立运行；无临时绝对路径和开发证书依赖。

## 9. 性能预算与测试

以下仅为**工程验收目标，非测量事实**：

| 指标 | 目标 | 测试方式 |
| --- | --- | --- |
| 菜单栏隐藏空闲总内存 | <100 MB | Activity Monitor / Instruments：含 WebKit 子进程 |
| 正常编辑内存 | <180 MB | 使用典型 Markdown 和代码块 |
| 空闲 CPU | 接近 0% | 静置 10 分钟并检查 wakeups |
| 热启动窗口显示 | P95 <150 ms | 快捷键到窗口可见/聚焦 |
| 1 万条数据检索 | P95 <100 ms | 用户输入至结果渲染完成 |
| 大文档 | 1 MB Markdown 仍可编辑 | 编辑器/预览压力测试 |

生成 10,000 条中文+英文+代码混合测试数据，记录 SQLite 查询耗时、Tauri IPC 耗时与 Vue 渲染耗时。列表分页、非必要组件延迟挂载、Shiki 语言按需加载、搜索防抖约 100–150 ms；不要为了省内存每次隐藏窗口都重建 WebView。

## 10. 测试矩阵

- **单元测试**：解析代码围栏、复制内容提取、标签规范化、FTS 查询转义、路径解析、窗口状态机。
- **数据库集成测试**：CRUD、外键级联、软删除/恢复、Migration、备份/还原、读写权限拒绝、损坏文件、schema 新旧版本、FTS 一致性。
- **Vue 组件测试**：左栏过滤/滚动、选中态、Markdown 阅读/编辑、主题跟随系统、保存失败提示、键盘访问。
- **macOS 手动/E2E**：全局快捷键冲突、菜单栏左右键、关闭隐藏、Dock 行为、双显示器与断连、Spaces/全屏/Stage Manager、置顶焦点、开机启动、文件同步目录切换。
- **数据安全**：恶意 Markdown HTML 和外链、异常长代码围栏、意外断电或强制结束后的数据恢复。

## 11. 任务拆解与依赖

| ID | 任务 | 依赖 | 验收产物 |
| --- | --- | --- | --- |
| T001 | 建立 Tauri/Vue 工程，设置 lint/test/build | - | 构建通过 |
| T002 | 原生菜单栏 + 单实例 | T001 | 仅一个实例 |
| T003 | 全局快捷键与窗口状态机 | T002 | 反复开关可靠 |
| T004 | 居中/置顶/尺寸记忆/多显示器 | T003 | 多屏验收 |
| T005 | SQLite 创建/打开/配置与迁移 | T001 | 读写成功 |
| T006 | CRUD/标签/事务/自动保存 | T005 | 集成测试 |
| T007 | 双栏 UI + NoteList/TagList | T006 | 真实数据联动 |
| T008 | Markdown 编辑器与预览 | T007 | 源码/预览切换 |
| T009 | 代码高亮与独立复制 | T008 | 复制正确 |
| T010 | FTS5 搜索与中文优化 | T006,T007 | 测试集指标 |
| T011 | 主题系统与偏好设置 | T007,T008 | 外观持久化 |
| T012 | 数据库迁移/备份/错误恢复 UI | T006,T007 | 故障演练 |
| T013 | 性能基准、稳定性测试 | T009,T010,T011,T012 | 性能报告 |
| T014 | macOS 打包、签名、使用说明 | T013 | 可运行安装包 |

## 12. 给 Codex 的执行指令模板

将本文件置于仓库 `docs/IMPLEMENTATION_PLAN.md` 后，使用以下指令：

> 你是该 macOS Tauri 2 + Vue 3 + Rust + SQLite 项目的实现 Agent。严格按照 `docs/IMPLEMENTATION_PLAN.md` 的范围、依赖关系和验收标准执行。先完成 M0 PoC，不要提前实现云同步、独立 Snippet 类型、第三栏或 Quick Panel。每个任务执行：代码实现 → 类型/单元测试 → 实机验证项列明 → 文档更新 → Git 提交。依赖或 API 与官方当前版本不一致时，以锁定版本的官方文档和编译结果为准，并在 `docs/DECISIONS.md` 记录取舍。不能完成的 macOS 真实交互测试必须明确标为待验证，不能假称已通过。M0 达标后按 T005–T014 顺序推进，允许不冲突的 UI/数据测试并行。每个里程碑更新完成项、测试结果和残留风险。

## 13. 参考文档

- Tauri 2 — [Global Shortcut](https://v2.tauri.app/plugin/global-shortcut/)
- Tauri 2 — [Plugins](https://v2.tauri.app/plugin/)
- Tauri 2 — [Window API](https://v2.tauri.app/reference/javascript/api/namespacewindow/)
- Tauri 2 — [Window Menu](https://v2.tauri.app/learn/window-menu/)
- Tauri 2 — [Configuration](https://v2.tauri.app/reference/config/)
- SQLite — [FTS5](https://www.sqlite.org/fts5.html)
- CodeMirror — [Language Data](https://github.com/codemirror/language-data)
