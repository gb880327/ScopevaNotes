## 1. 工程与规格基线

- [x] 1.1 锁定技术栈与版本：Tauri 2、Vue 3.5、TypeScript 5.9（vue-tsc peer 约束）、Vite 8、Tailwind 4、rusqlite 0.40（bundled + FTS5）。
- [x] 1.2 建立目录骨架 `src/`、`src-tauri/src/{commands,models,services,repositories,database,platform/macos}`、`migrations/`、`tests/`。
- [x] 1.3 配置 `tauri.conf.json`（identifier、窗口 1100×720 / 最小 760×480、CSP、bundle、macOS 12+）与 `capabilities/default.json`。
- [x] 1.4 校验链路可跑通：`cargo check`、`cargo test`、`pnpm vue-tsc --noEmit`、`pnpm vitest run`、`pnpm vite build`。
- [x] 1.5 运行时冒烟：`pnpm tauri dev` 无 panic 启动，默认路径生成 `notes.db` 并应用 v1/v2 迁移。
- [x] 1.6 提交初始骨架（`f42ab93`、`1234a06`）并补齐 `README` / `docs`。
- [x] 1.7 删除 docs 下已迁移的规划文档，只保留 `ARCHITECTURE.md` 作代码级参考。

## 2. 笔记数据层（notes-core）

- [x] 2.1 `notes` / `tags` / `note_tags` 表与索引，UUIDv7 + 毫秒时间戳。
- [x] 2.2 摘要列表：摘要字段、收藏优先 + 更新时间倒序、游标分页（50 条）、批量取标签避免 N+1。
- [x] 2.3 详情、创建、更新、收藏切换、软删除、恢复。
- [x] 2.4 乐观并发：校验 `expectedUpdatedAt`，冲突返回 `Conflict` 且不写入。
- [x] 2.5 标签归一化、大小写不敏感唯一、带计数列表、重命名、删除（保留笔记）。
- [x] 2.6 自动保存：600ms 防抖 + 串行队列 + 版本号，切换/隐藏/换库前 flush。
- [x] 2.7 集成测试覆盖 CRUD、冲突、软删除移除索引、恢复、标签大小写重复。
- [x] 2.8 补齐回收站入口：列出已删除笔记并支持恢复（`restore_note` 已有命令，UI 未暴露）。
- [x] 2.9 保存冲突的界面处理：提示并提供「重载磁盘版本 / 保留本地并强制覆盖」。
- [x] 2.10 数据层压测：1,000 条随机笔记的增删改查耗时与列表分页正确性。`tests/crud_bench.rs`（`cargo test --release --test crud_bench -- --ignored --nocapture`，1,000 条确定性随机笔记 + 20 个标签，debug 构建 3.1 s 跑完）。实测：create P50 1.97 / P95 2.72 ms，get P50 0.09 / P95 0.11 ms，update P50 2.33 / P95 3.02 ms，列表每页 50 条 P50 2.35 / P95 2.69 ms，搜索（1k 库）P50 4.75 / P95 5.87 ms，soft_delete ×100 200 ms，restore ×50 105 ms，库体积 2.6 MB。分页断言：20 页取回全部 1,000 条，无重复、无遗漏，且严格符合 `updated_at DESC, id DESC`（同毫秒内靠 id 兜底）。

## 3. 编辑与预览（markdown-editing）

- [x] 3.1 CodeMirror 6 + `markdown()`，字号/主题跟随外观设置。
- [x] 3.2 markdown-it 预览，`html: false`；外链经 opener 打开。
- [x] 3.3 围栏切分与逐块渲染，12 个单元测试覆盖语言识别/无围栏/未闭合/`~~~`/长围栏/信息串。
- [x] 3.4 代码块复制只写代码原文；标题区复制全文为原始 Markdown。
- [x] 3.5 `Command+E` 切换读/写、`Esc` 退出并 flush。
- [x] 3.6 接入 Shiki 高亮：`highlight.ts` 用 `createHighlighterCore` + JS 正则引擎，42 种语言与常见别名（`ts`/`sh`/`yml`/`c++`…）按需动态 import，每种语言独立 chunk；未知语言/空代码/超 2 万字符/语法加载失败一律回 `null` 降级纯文本，不抛异常。浅色用 `github-light-high-contrast`（实测原 `github-light` 橙色字在代码背景上仅 3.2:1），深色用 `github-dark`，两种主题全部 token 实测对比度 ≥ 5.67:1；`resolvedTheme` 提到 settings store，代码块随主题重新着色。
- [x] 3.7 预览安全用例：`markdown.ts` 改为协议白名单（`http`/`https`/`mailto`/相对路径），`validateLink` 先剥离控制字符再判 scheme，拒绝 `//host`；预览点击统一 `preventDefault`，只有 http(s) 交给 opener。`fences.ts` 改行扫描，按 CommonMark 规则处理长于开围栏的闭合、信息串内反引号、未闭合围栏到文末（可复制）、5 万字符行。`markdown.test.ts` + `fences.test.ts` 共 30 个用例覆盖 `<script>`、`<img onerror>`、`javascript:`/大小写混写/控制字符/`data:`/`file:`/`//host`、未闭合与超长围栏。
- [x] 3.8 组件测试：`NoteHeader.test.ts` 读/写切换（含退出前 flush）、复制全文文案、收藏按钮、标签渲染；`CodeBlock.test.ts` 复制原文、`复制代码→已复制→复制代码`、失败文案、未知语言/空语言纯文本；`NotePreview.test.ts` 空笔记、围栏独立复制、未闭合围栏可复制、脚本不执行、危险链接无锚点、https 链接走系统浏览器。前端测试共 74 个用例全绿（原 12 个）。

## 4. 搜索（search）

- [x] 4.1 `notes_fts`（FTS5 unicode61）表与 `reindex_note`，在创建/更新/软删除/恢复/标签变更的同一事务内维护。
- [x] 4.2 查询输入转义（`%`、`_`、`\`），标签 + 收藏 + 关键词组合过滤。
- [x] 4.3 150ms 防抖输入，过期响应丢弃。
- [x] 4.4 列表查询接入 `notes_fts MATCH` 粗筛 + 字面 `LIKE` 精筛（`note_repo::list`）。粗筛表达式由 `search_index::to_match_query` 生成，只允许引号短语与前缀运算符进入 FTS 解析器。排序仍按 `updated_at` 游标（保证分页稳定，不按 bm25 重排）。
- [x] 4.5 中文策略：索引侧逐字展开 CJK（`explode_cjk`，迁移 v3 重建投影），查询侧双字及以上生成相邻字符短语，因此无需 LIKE 全表回退；输入不含字母数字（如 `++`）时跳过 `MATCH`，仅走字面 `LIKE`。
- [x] 4.6 基准 `src-tauri/tests/search_bench.rs`（`cargo test --release --test search_bench -- --ignored --nocapture`）：10,000 条中英混合，库 14.6 MB（含 FTS 投影），首屏 P50 16.5 ms / P95 18.5 ms / 最差 19.7 ms（42 次查询，含零命中与中文双字），索引全量重建 1.07 s。
- [x] 4.7 规格场景测试（`src-tauri/tests/database.rs`）：正文命中、标签命中、`100%` 与 `1000` 不混淆、`foo AND "bar` 不报错、中文双字与词组嵌长句、英文前缀、删除后不可检索、恢复后立即可检索、关键词 + 标签/收藏交集、清空关键词保留筛选。

## 5. 笔记库（library）

- [x] 5.1 默认库路径 `Application Support/com.scopeva.notes/notes.db`。
- [x] 5.2 事务化迁移执行器、`schema_migrations`、拒绝更高版本 schema。
- [x] 5.3 打开既有库：路径校验、`quick_check`、失败返回错误。
- [x] 5.4 新建库拒绝覆盖既有文件；备份用 `VACUUM INTO` 且拒绝覆盖。
- [x] 5.5 设置页展示当前路径，提供打开/新建/备份入口，操作前 flush。
- [x] 5.6 路径失效时的界面告知与「重新选择 / 使用默认库」入口。后端 `LibraryInfo` 新增 `notice: { kind: 'fallback' | 'cloud-sync', message, path }`；`resolve()` 拆成 `resolve()`（读配置）+ `resolve_from(configured, default_path)`，配置指向的文件不存在/损坏时**回退默认库并附 fallback 提示**（不再是只写 `eprintln!`），启动不因此失败。前端设置页顶部 `role="status"` `aria-live="polite"` 横幅显示原因与被丢弃的路径，提供「重新选择…」（调文件选择器）与「使用默认库」（`dismiss_library_notice(use_default=true)`，仅 fallback 时重写 `config.json`，云同步提示下忽略该参数以免丢弃用户路径）。回归：`tests/library.rs` 6 个用例覆盖失效路径回退、损坏文件回退（提示含「无法打开」）、正常配置无提示、五类云目录识别、云库提示、锁冲突报 `busy`。
- [x] 5.7 笔记库统计：笔记数量与文件大小。
- [x] 5.8 云同步目录提示，以及外部修改/锁冲突时的可见错误。①云同步：`cloud_sync_provider()` 按**路径组件**识别 Dropbox / OneDrive（含 `OneDrive-Acme`）/ Google Drive（含 `GoogleDrive-me`）/ iCloud Drive（含 `Mobile Documents`）/ Nextcloud / ownCloud / pCloud / Sync.com / Box，只对实际会带后缀的 provider 允许 `name-…`/`name …` 前缀匹配，避免 `dropbox-notes.db` 误判；命中后 `LibraryInfo.notice` 带 `cloud-sync` 提示（同一时间只在一台设备写入），设置页展示但不阻止使用。②锁冲突：`AppError::Busy`（`kind = "busy"`）+ `AppError::classify()` 把含 `locked`/`busy` 的 SQLite 错误提升为 `Busy`，连接 `busy_timeout` 参数化为 5 s（测试用 150 ms）；前端 `errorMessage()` 按错误 kind 给出可读文案，自动保存失败仍走既有 `notes.error` 横幅（工具栏状态显示「保存失败」），不静默丢弃。③外部修改：写路径沿用 `expected_updated_at` 乐观锁，冲突返回 `Conflict` 并弹「重新载入 / 保留我的」，`force` 仅由用户显式选择触发。
- [x] 5.9 迁移/备份失败演练：只读目录、损坏文件、迁移中断、版本过新。`tests/failures.rs` 6 个用例（`--test-threads=1`）：①只读目录打开库 → 返回 `Database`/`Io` 错误且**不留下空文件**；②备份到只读目录 → 失败、无残留目标文件、源库仍可正常列出笔记；③备份目标已存在 → `Invalid` 且原文件字节不变；④4 KB 垃圾文件 → `Database` 错误而非「新建空库」；⑤`schema_migrations` 写入 999 → `Invalid` 且消息点明「newer」；⑥半成品 `notes` 表让 0001 在 `CREATE INDEX` 阶段失败 → 单版本事务整体回滚（`tags`/`note_tags` 不存在、`schema_migrations` 0 行），与规格「迁移中途失败」一致。

## 6. macOS 外壳（macos-shell）

- [x] 6.1 单实例（`tauri-plugin-single-instance`），重复启动唤出已有窗口。
- [x] 6.2 `WindowManager` 串行化显隐状态机，`visible` 仅在成功后更新。
- [x] 6.3 显示时恢复尺寸 → 钳制到鼠标所在显示器 → 居中 → 置顶 → 聚焦；隐藏取消置顶。
- [x] 6.4 关闭按钮 `prevent_close` 改为隐藏；仅菜单栏退出结束进程。
- [x] 6.5 菜单栏菜单：显示/隐藏、新建、笔记库、设置、退出；`ui://*` 事件接前端。
- [x] 6.6 全局快捷键默认 `CommandOrControl+Shift+D`，注册失败不阻断启动。
- [x] 6.7 左栏拖拽调宽 220–420px，受窗口宽度约束并持久化。
- [x] 6.8 显示器居中单元测试（负坐标副屏、超大窗口钳制）。
- [ ] 6.9 实机验收：连按快捷键 100 次、多显示器、拔屏、Spaces/全屏/Stage Manager、Dock 与 ⌘Q 行为。
- [x] 6.10 快捷键可配置：设置界面录制组合键、持久化并在启动时重新注册；冲突时回退默认并提示，旧快捷键保持有效。
- [x] 6.11 菜单栏图标换为品牌 logo（`icons/tray-logo.png`，88px 源图按 18pt 渲染）。
- [x] 6.12 开机启动开关接入设置界面并反映系统实际状态。
- [x] 6.13 启动静默：`setup` 不再主动 `show` 主窗口；隐藏状态下仍完成 hydrate 与快捷键注册；首次运行说明改由菜单栏侧给出。
- [x] 6.14 退出确认：拦截菜单栏「退出」/`⌘Q`/`ExitRequested`，先让前端 flush 再弹确认；取消保持运行，保存失败不退出（前端事件 + Rust 对话框握手）。

## 7. 外观（appearance）

- [x] 7.1 设计令牌：浅/深色变量、5 种强调色、字号变量。
- [x] 7.2 主题（浅色/深色/跟随系统）、强调色、编辑器字号设置项。
- [x] 7.3 偏好写 Tauri Store；浏览器开发环境退回 localStorage；损坏时回退默认值。
- [x] 7.4 跟随系统即时切换与对比度检查（正文/次要文字/代码块/选中态）。系统外观变化已在真实浏览器验证（`matchMedia` 变化 → `data-theme` 切换 → 正文/侧栏/代码着色同步换色，再切回原样）。对比度：新增 `src/styles/tokens.test.ts`，把 `:root`、`[data-theme='dark']`、四种强调色覆盖层解析成真实变量表，半透明表面（`--bg-input`/`--selected`/`--accent-soft`）先与 `--bg` 合成，再对**每个文本令牌 × 每个表面 × 两种主题 × 五种强调色**断言 ≥ 4.5:1（13 个用例）。据此修掉四处真问题：①`--text-faint` 白底仅 3.26:1（侧栏 2.99）→ 浅色 `#68686e`（最差 4.61）、深色 `#a3a3a9`（最差 4.59）；②`--text-muted` 在选中底 `--selected` 上 4.12:1 → 浅色 `#5f5f66`，同时把 `--selected` 由 `0.07` 降到 `0.05` 黑；③强调色文字（链接、恢复链接、收藏星）浅色 3.65 / 深色 3.82（`--bg-elevated`）不达标 → 拆出 `--accent-text`（浅色蓝 `#0a66c2`、深色 `#54a6ff`，另为紫/绿/橙/石墨各配一档），`--accent` 只保留给填充、圆点、选中底色；④主按钮 `白字 on --accent` 仅 3.6:1 → 改 `background: --accent-text` + `--accent-contrast`（浅色白字、深色近黑），五种强调色下均 ≥ 5.0:1。另修深色 `--danger #ff453a` 在抬升/选中底只有 3.87:1 → `#ff7a72`。真实 DOM 复核：渲染 Sidebar + ContentPane + SettingsDialog，遍历 1260 个有文本的元素按合成后的实际颜色计算比值，浅/深 × 5 强调色 × 阅读/编辑/设置面板**零违规**（并注入 1.16:1 探针验证审计确实能报错）。未纳入断言：`--accent` 作为非文字填充（绿/橙圆点对白底 2.0:1）与 `--border` 边框对比度——圆点旁必有文字标签、边框属装饰性分隔，如需严格 WCAG 1.4.11 需另设 3:1 变体（已记入 design D18）。
- [x] 7.5 CodeMirror 与 Shiki 主题随外观设置联动。`features/notes/editorTheme.ts` 把编辑器拆成「主题无关的 chrome」+「主题相关的语法色」：chrome（行号、选区、搜索框、括号、折叠占位）全部走设计令牌，语法色用 `Compartment` 在 `resolvedTheme` 变化时 `reconfigure`，不销毁重建 EditorView（内容与光标位置保留）。调色板对齐预览的 Shiki 主题但针对编辑器背景（`--bg` 而非 Shiki 自带底色）重调，`editorTheme.test.ts` 从 `tokens.css` 实测 `--bg` 并逐色断言 ≥ 4.5:1；实测浅色最差 5.04、深色 5.53，行号由 `--text-faint`（白底 3.3:1，不达标）改用 `--text-muted`（5.07 / 6.61）。`useTheme.test.ts` 验证强制浅/深、跟随系统、系统外观变化即时回调、设置变更即时生效、卸载后停止监听。

## 8. 质量与可验证性

- [x] 8.1 Rust 单元测试（窗口几何）、集成测试（数据层）、前端单元测试（围栏）。
- [x] 8.2 组件测试：左栏过滤、空态、键盘可达性。`src/components/layout/Sidebar.test.ts` 9 个用例：三个筛选 chip 的初始 `active` 与「全部」默认态、收藏切换会经 store 刷新列表、回收站视图与笔记列表互换（含 `aria` 结构由 `role="listbox"` 存在性体现）、空列表「没有匹配的笔记」+「清除筛选」、空回收站「回收站是空的」、标签列表含「全部标签」且点击写入 `activeTagIds` 并刷新、所有操作元素都是 `type="button"` 的真实按钮（无 `tabindex="-1"`）、Enter/Space 与 click 等价、搜索框 `type="search"` 可聚焦且带说明性 placeholder。滚动本身由 CSS `overflow-y: auto` 承担，无 JS 分支可断言，未纳入测试。
- [ ] 8.3 端到端冒烟：新建 → 编辑 → 自动保存 → 搜索 → 删除 → 恢复。**未完成**：本机无法驱动 Tauri 的 WKWebView（无「辅助访问」与「屏幕录制」权限，`osascript` 报 `-25211`；`tauri-driver` 在 macOS 也不支持 WKWebView）。当前替代覆盖：8.5 的 `tests/recovery.rs` 走完「写入 → 崩溃 → 恢复」全链路，`tests/crud_bench.rs` 覆盖 1,000 条笔记的增删改查与分页，`Sidebar.test.ts` 覆盖左栏交互；缺的是 UI 点击到 IPC 的那一层。需人工在真机按 `docs/RELEASE.md` 的步骤跑一遍后勾选。
- [ ] 8.4 24 小时驻留的资源与空闲 CPU 测量；冷/热启动耗时记录。**部分完成**：新增 `scripts/perf-check.sh`（`startup` / `soak <hours>`），并在 release 通用包上实测。启动里程碑（x86_64，重复两次）：进程出现 0.24 s / 0.24 s，笔记库打开 0.84 s / 0.83 s，WebKit 内容进程 1.11 s / 1.14 s——冷热几乎相同，因为二进制与 WebKit 框架都在页缓存里（真正的冷启动需要 `sudo purge`，本机未做）。空闲采样 10 分钟（每 20 s，`ps -o time=` 差分）：CPU 全程 0.00%，RSS 52.3 → 41.7 MB 后稳定，无增长趋势。**24 小时整段未运行**，故不勾选；待用户执行 `scripts/perf-check.sh soak 24` 后补记。
- [x] 8.5 断电/强杀恢复演练与备份还原往返。`tests/recovery.rs` 2 个用例。①真实硬杀：用 `std::process::exit(42)` 启动自身子进程，一种模式在事务提交 20 条后直接退出，另一种模式 `BEGIN` 后插入 50 条不提交就退出——两者都不跑析构、不 checkpoint。父进程复查时确认 `notes.db-journal` 确实残留（打印 `journal after crash: true`），重开后 `PRAGMA quick_check` 为 `ok`，20 条已提交笔记全部存在，50 条未提交记录为 0 条（SQLite 热日志回滚生效）。②备份还原往返：30 条笔记（含 1 个标签、10 条带标签、15 条含 `sqlite`）+ `VACUUM INTO` 快照，删除真实库文件后用快照覆盖回去，重开校验 `quick_check`、笔记总数 30、搜索命中仍为 15、标签计数总和 10——FTS 投影随快照一起还原。注：当前 `journal_mode` 是 SQLite 默认的 rollback journal（设计 D10 明确「不依赖 WAL 文件」），因此演练以 `-journal` 热日志为准。
- [ ] 8.6 把本清单中未勾选的验收项与本 change 的规格场景逐条对齐，全部通过后才能勾选。**对齐已完成，尚不能勾选**：`verification.md` 把 7 个 capability / 24 条 Requirement / 100 个 Scenario 逐条映射到证据，结论是 79 个有自动化测试、3 个本机人工确认、18 个待人工——待人工项与 6.9 / 8.3 / 8.4 / 9.2 四个未勾选任务一一对应，没有孤儿场景。核对时补了三处真实缺口：① `refresh()`/`loadMore()` 缺请求序号守卫，慢查询回包会覆盖新查询结果（已加 `listSeq` + `src/stores/notes.test.ts` 3 个用例）；② 自动保存队列无单测（已加 `src/composables/useAutosave.test.ts` 5 个：去抖合并、串行、失败保留 dirty、discard、空 flush）；③ 设置持久化只覆盖了 localStorage 回退（已加 `src/stores/settings.desktop.test.ts` 走 mock 的 Tauri Store，以及损坏文件回退的用例）。等这 18 个场景在真机人工通过后，才能连同 6.9 / 8.3 / 8.4 / 9.2 一起勾选。

## 9. 发布（release-readiness）

- [x] 9.1 发布构建产出可运行 `.app`（Intel 与 Apple Silicon）。`pnpm tauri build` 产出 x86_64 `Notes.app`（18.23 MiB）+ `Notes_0.1.0_x64.dmg`（8.17 MiB）；`rustup target add aarch64-apple-darwin` 后 `pnpm tauri build --target universal-apple-darwin` 产出通用包（34.78 MiB）+ `Notes_0.1.0_universal.dmg`（13.50 MiB）。`lipo -info` 确认 fat 文件同时含 `x86_64 arm64`。x86_64 包已在本机实际启动（`open -a` → 进程、库文件、WebKit 内容进程全部就位，见 8.4）。**arm64 slice 只做了产物结构校验，未在 Apple Silicon 实机运行**；通用包目前是 ad-hoc 签名。
- [ ] 9.2 Developer ID 签名 + 公证流程与校验（对外分发前完成）。
- [x] 9.3 发布文档：安装、快捷键、库位置与备份、已知限制；补 `CHANGELOG`。新增 `docs/RELEASE.md`（构建命令与四类产物路径、Gatekeeper 放行、首次启动静默与一次性提示、快捷键表、库位置/指针配置/备份与云同步注意、已知限制、卸载含 LaunchAgent 残留、开发期自检命令）与根目录 `CHANGELOG.md`（0.1.0：笔记与编辑、预览、搜索、笔记库、macOS 外壳、外观、质量与性能、已知限制）。README 补 `docs/RELEASE.md` / `CHANGELOG.md` 链接、通用包构建命令与 `scripts/perf-check.sh` 用法。
- [x] 9.4 清理开发残留：移除临时绝对路径、调试输出与未使用依赖。移除未使用的直接依赖：`reka-ui`、`@codemirror/commands`（仅由 `codemirror` 的 `basicSetup` 间接引入）、`@tauri-apps/plugin-global-shortcut`（快捷键只在 Rust 侧注册）；`pnpm remove` 后 lockfile 同步。`grep` 复核：`src/`、`src-tauri/src/` 无 `/Users/…` 或 `/tmp/…` 绝对路径（唯一命中是组件测试里的假数据 `'/tmp/notes.db'`），无 `console.log`/`dbg!`。保留的 6 处 `eprintln!` 是启动回退、快捷键注册失败、窗口/托盘/开机启动失败的诊断信息——没有引入 `log` crate 前它们是唯一的失败线索，属有意保留而非调试残留。临时审计 fixture（`preview-audit.html`、`src/__audit-fixture.ts`）此前已删，`git status` 干净。
