# Changelog

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

- 未签名、未公证；arm64 slice 未在 Apple Silicon 实机运行。
- 24 小时驻留、端到端 UI 冒烟、macOS 实机交互验收（6.9/8.3/8.4）未完成。
- 无独立代码片段实体、无多窗口、无快速面板（v1 范围如此）。

详见 `docs/RELEASE.md` 与 `openspec/changes/v1-notes-manager/tasks.md`。
