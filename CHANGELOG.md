# Changelog

## 0.2.0 — 未发布

### 批量导入 Markdown

- 菜单栏「导入 Markdown…」与设置里的同一入口，三阶段模态：选择目录 → 进度 → 报告。
- 只读扫描 + dry-run 预览（待导入 / 已导入跳过 / 隐藏条目 / 本地文件引用 / 无法读取）。
- 递归发现 `.md`/`.markdown`，跳过隐藏目录（`.obsidian`、`.trash`）、`node_modules` 与符号链接并计数。
- 解码：UTF-8 优先（去 BOM），非法时 GB18030 兜底并在报告里逐条计数。
- frontmatter 子集解析（`title` / `tags` / `created`，支持 inline list、逗号串、block list）；
  title 回退首个 H1，再回退文件名。
- 时间戳取文件系统元数据（`mtime` → `updated_at`，`birthtime` → `created_at`），未来时间钳到当前，
  导入后的列表顺序与原目录的修改时间一致。
- 正文逐字保真：只剥离 frontmatter，`[[链接]]`、`==高亮==`、`%%注释%%`、callout、`![](...)` 全部原样保留。
- 200 条/批提交、`ui://import-progress` 进度事件、批间响应取消（已提交批次保留），
  重跑同一目录按 `source_path` 幂等跳过。
- 报告列出逐文件失败原因（含路径）、按 GB18030 解码数、未迁移的本地文件引用数、耗时与本批标签。
- 「删除本批」按 `import-YYYY-MM-DD` 标签整批软删除（进回收站，可恢复）。
- 性能（release，20 目录 / 每篇 ~600 字中文）：1,000 条 0.49 s；5,000 条 2.12 s，
  单批最长间隔 131 ms（导入期间自动保存的最坏等待上界，远低于 5 s `busy_timeout`）。
  这一项顺带修掉了 `notes_fts` 的 O(n²) 写入：新笔记走 append-only 的 `index_new_note`（设计 D14）。
- 已知限制：不迁移附件、不解析 Obsidian 扩展语法、不做同步/目录监听、同日批次共用撤销标签。

## 0.1.0 — 未发布

首个可运行版本。Tauri 2 + Vue 3.5 + SQLite（rusqlite bundled，FTS5）。

### 笔记与编辑

- 笔记 CRUD、软删除、回收站与恢复；标题/正文/tags 自动保存（带队列与失败状态）。
- 保存冲突走 `expected_updated_at` 乐观锁，提供「重新载入 / 保留我的」。
- CodeMirror 6 编辑器：Markdown 语法高亮、主题与字号跟随设置、语法色随深浅色即时切换。

### 预览

- 自建 CommonMark 围栏切分 + markdown-it（`html: false`）+ Shiki 按需高亮
  （42 种语言，按语言拆 chunk，未知语言/超长代码退回纯文本）。
- 代码块复制按钮；未闭合围栏仍按代码块渲染且可复制。
- 链接白名单（http/https/mailto/相对路径），点击不导航 webview。

### 搜索

- FTS5 粗筛 + 字面 `LIKE` 精筛；中文逐字索引（`explode_cjk`），`100%` 不会命中 `1000`。
- 游标分页（`updated_at DESC, id DESC`），10k 条库实测 P50 16.5 / P95 18.5 ms。

### 笔记库

- 默认库、自定义路径、新建、迁移（快照后切换，旧库保留）、`VACUUM INTO` 备份且拒绝覆盖。
- 事务化迁移执行器、`schema_migrations`、拒绝更高版本 schema。
- 路径失效时回退默认库并显示可见提示（重新选择 / 使用默认库）。
- 云同步目录识别（Dropbox/OneDrive/Google Drive/iCloud Drive/Nextcloud 等）与建议性提示。
- 数据库锁冲突映射为 `busy` 错误并给出可读文案。

### macOS 外壳

- 单实例、菜单栏常驻（品牌 logo）、关闭即隐藏、托盘退出、开机启动开关。
- 全局快捷键可自定义（默认 `⇧⌘D`），注册失败回退默认并提示。
- 启动静默（不主动显示主窗口）、首次运行一次性原生提示、退出确认（先 flush 再确认）。
- 窗口显示时恢复尺寸 → 钳制到当前显示器 → 居中 → 置顶，隐藏时取消置顶。

### 外观

- 浅色 / 深色 / 跟随系统，5 种强调色，字号可调。
- 设计令牌拆分「填充」与「文字」（`--accent` vs `--accent-text`），
  `src/styles/tokens.test.ts` 对 5 个文本令牌 × 7 个表面 × 2 主题 × 5 强调色断言 ≥ 4.5:1。

### 质量与性能

- 数据层压测（1,000 条）：create P95 2.7 ms、update P95 3.0 ms、分页每页 P95 2.7 ms。
- 崩溃恢复演练（真实 `process::exit` 子进程）：已提交写入存活、未提交事务回滚。
- 备份还原往返保留笔记、标签计数与 FTS 投影。
- 失败演练：只读目录、损坏文件、迁移中断、版本过新、备份目标已存在。
- 启动里程碑（本机 x86_64）：进程 0.19 s、库打开 0.74 s（`scripts/perf-check.sh startup`）。
- 10 分钟空闲采样：0.00% CPU、RSS 41.7 MB。

### 已知限制

- v1 只到本机：ad-hoc 签名，不做 Developer ID 签名与公证（9.2 移出范围，D20）。
- arm64 slice 只做了产物结构校验，未在 Apple Silicon 实机运行。
- macOS 实机交互、端到端 UI 冒烟、24 小时驻留由用户接受、未实机验证
  （6.9/8.3/8.4，D20）；真机出问题先查 `verification.md` 对应场景。
- 无独立代码片段实体、无多窗口、无快速面板（v1 范围如此）。

详见 `docs/RELEASE.md` 与 `openspec/changes/archive/2026-10-24-v1-notes-manager/tasks.md`。
