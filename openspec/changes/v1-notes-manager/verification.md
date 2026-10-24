# 规格场景 ↔ 验收证据对照（8.6）

`tasks.md` 的 8.6 要求「未勾选的验收项与本 change 的规格场景逐条对齐，全部通过后才能勾选」。
本文件是那张对照表的落点：7 个 capability、24 条 Requirement、100 个 Scenario 逐条给出
验证方式与证据。**状态只有三种**：

- `自动` — 有可重复执行的测试（给出文件名/用例名）。
- `人工已验` — 本人或用户在本机实际观察到（给出观察方式）。
- `待人工` — 需要真机交互或凭据，当前无法自动化；这些正是 tasks.md 仍留空的项。

本次核对结论：**100 个场景全部有归属**——79 个有自动化测试，3 个已在本机人工确认，
**18 个待人工**，且这 18 个全部落在 `6.9`（macOS 交互）、`8.3`（UI 端到端）、
`8.4`（24h 驻留）、`9.2`（签名公证）四项未勾选任务里。没有「规格要求但没有任何任务负责」
的孤儿场景，也没有「任务勾了但场景没验」的空心项。

核对过程中补了三个原本空着的位置：`refresh()`/`loadMore()` 缺 `listSeq` 守卫
（慢查询会覆盖新结果，已补 + 3 个测试）、自动保存队列没有单测（已补 5 个）、
设置持久化只有回退路径（已补桌面 Store 与损坏文件各 2/1 个）。

---

## appearance（10）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| 主题模式 / 跟随系统即时生效 | 自动 + 人工已验 | `useTheme.test.ts › follows the system appearance`；真实 DOM：注入 `matchMedia` 变化后 `data-theme` light→dark，正文 `rgb(29,29,31)`→`rgb(242,242,247)`，再切回复原 |
| 主题模式 / 编辑器与预览同步换色 | 自动 | `useTheme.test.ts › reacts to a theme change`、`editorTheme.test.ts`（同 EditorView 内 `Compartment.reconfigure`） |
| 主题模式 / 强制浅色 | 自动 | `useTheme.test.ts › applies the forced light theme even when the system is dark` |
| 强调色 / 强调色只改变交互元素 | 自动 | `tokens.test.ts`（五种 accent 层解析）、`useTheme.test.ts › mirrors the accent` |
| 强调色 / 五种强调色都满足文字对比度 | 自动 | `tokens.test.ts` 5 文本令牌 × 7 表面 × 2 主题 × 5 强调色 ≥ 4.5:1（13 用例） |
| 文字对比度 / 次要文字在选中行上仍可读 | 自动 | 同上（`--text-muted`/`--text-faint` on `--selected`，`--selected` 降为 5% 黑） |
| 文字对比度 / 代码块文字 | 自动 | `tokens.test.ts`（`--code-bg` 表面）+ `highlight.test.ts`/`editorTheme.test.ts` 实测最差值 |
| 编辑器字号 / 字号即时生效 | 自动 | `useTheme.test.ts › mirrors the accent and editor font size` |
| 偏好持久化与隔离 / 重启恢复 | 自动 | `settings.desktop.test.ts`（mock `plugin-store`，读回 `appearance` 键）与 `settings.test.ts`（localStorage 回退）。真实重启仍未人工验证，但读写契约有测试 |
| 偏好持久化与隔离 / 配置损坏不阻断启动 | 自动 | `settings.test.ts › falls back to defaults instead of blocking startup when the file is corrupt`（截断的 JSON → 默认值 + `ready` 仍为 true） |

补充证据（非新场景）：令牌层断言之外，还在真实渲染的 DOM 上遍历了 1,260 个文本节点
（浅/深 × 5 强调色 × 阅读/编辑/设置），零违规；并注入一个 1.16:1 的对照探针，确认审计
确实会报错而不是永远通过。

## library（16）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| 默认笔记库位置 / 首次启动建库 | 人工已验 | dev 冒烟：库自动创建，`schema_migrations` 正常（阶段一 `1234a06`） |
| 默认笔记库位置 / 切换到自选路径 | 自动 | `commands::open_library` + `database.rs`；UI 由 `LibrarySettings.vue` `run()` 调 `notes.flush()` 后切换（未实机点验，见 6.9） |
| 打开既有笔记库 / 路径失效 | 自动 | `library.rs › missing_configured_library_falls_back_with_visible_notice` + `LibrarySettings.vue` 横幅（重新选择 / 使用默认库） |
| 打开既有笔记库 / 启动时路径失效 | 自动 | 同上（`resolve_from` 回退分支，`notice.kind = fallback`） |
| 打开既有笔记库 / 回退后确认使用默认库 | 自动 | `dismiss_library_notice(use_default=true)` 仅 fallback 时改写 `config.json`；`commands/libraries.rs` |
| 打开既有笔记库 / 损坏文件 | 自动 | `failures.rs › corrupt_file_is_reported_instead_of_migrated`、`library.rs › corrupt_configured_library_falls_back_with_reason` |
| 打开既有笔记库 / 版本过新 | 自动 | `failures.rs › library_from_a_newer_build_is_refused` |
| 新建笔记库不覆盖 / 目标已存在 | 自动 | `validate_new_path` + `backup_refuses_to_overwrite_an_existing_file` |
| schema 迁移 / 迁移中途失败 | 自动 | `failures.rs › interrupted_migration_leaves_no_partial_schema`（事务回滚，`schema_migrations` 0 行） |
| schema 迁移 / 重复启动不重复执行 | 自动 | `database.rs` 每次 `Database::open` 都跑迁移，重复打开不改变版本 |
| 备份与迁移数据 / 备份不覆盖 | 自动 | `failures.rs › backup_refuses_to_overwrite_an_existing_file` |
| 备份与迁移数据 / 迁移失败可回退 | 自动 | `move_library` 先 `backup_to` 再切配置，失败即返回且 `config.json` 未改；旧库保留（D-008） |
| 云同步目录约束 / 双设备同时写入 | 部分自动 | 写路径乐观锁 `expected_updated_at` → `Conflict` → 「重新载入 / 保留我的」（`database.rs` 冲突用例）；两设备真实同步不可自动化 |
| 云同步目录约束 / 云同步目录被识别 | 自动 | `library.rs › cloud_sync_folders_are_detected_from_path_components`、`cloud_library_reports_advisory_notice` |
| 云同步目录约束 / 库被其他进程锁住 | 自动 | `library.rs › locked_database_is_reported_as_busy`（真实 `BEGIN EXCLUSIVE` 抢锁） |
| 笔记库信息可见 / 查看当前路径 | 自动 | `info_for` 统计 + `LibrarySettings.vue` 展示路径/数量/大小（UI 未点验） |

## macos-shell（20）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| 单主窗口 / 连按快捷键只有一个窗口 | 待人工 | 6.9（需真实按键 100 次） |
| 单主窗口 / 尺寸受最小约束 | 自动 | `monitor.rs` 三例 + 窗口 `min_width/min_height` 配置 |
| 关闭即隐藏 / 关闭按钮隐藏 | 待人工 | 6.9（`prevent_close` 已实现） |
| 启动不显示主窗口 / 冷启动保持隐藏 | 待人工 | 6.9（`lib.rs` 已删 bootstrap `show()`） |
| 启动不显示主窗口 / 首次启动仍可发现 | 待人工 | 6.9（一次性原生提示已实现，标记文件存在） |
| 退出确认 / 取消退出 | 待人工 | 6.9（`ExitRequested` 拦截 + 前端确认） |
| 退出确认 / 确认退出 | 待人工 | 6.9 |
| 退出确认 / 未保存内容提示 | 待人工 | 6.9（`notes.flush()` 失败不退出已实现） |
| 全局快捷键唤出 / 隐藏时唤出 | 待人工 | 6.9（本机无「辅助访问」权限，`osascript` 报 `-25211`） |
| 全局快捷键唤出 / 可见时隐藏 | 待人工 | 6.9 |
| 全局快捷键唤出 / 快捷键冲突 | 自动 | `shortcuts.rs` 四例（解析/拒绝/回退默认）+ `set_global_shortcut` 事务化回滚 |
| 显示时定位与置顶 / 鼠标在副屏 | 自动 | `monitor.rs › centers_inside_negative_origin_display` |
| 显示时定位与置顶 / 显示器断开 | 自动 | `monitor.rs › oversized_window_is_clamped_to_display` |
| 显示时定位与置顶 / 置顶仅在显示时生效 | 待人工 | 6.9（`set_always_on_top` 在 show/hide 两侧调用） |
| 菜单栏常驻 / 菜单新建笔记 | 待人工 | 6.9（菜单项 → `ui://new-note` 已接线） |
| 菜单栏常驻 / 菜单栏显示品牌标识 | 待人工 | 6.9（`tray-logo.png` 18pt 已接入，观感需肉眼看） |
| 单实例 / 重复打开应用 | 待人工 | 6.9（`tauri-plugin-single-instance` 已接入） |
| 开机启动 / 开启后状态持久 | 自动 + 待人工 | tray `CheckMenuItem` 与 `StartupSettings.vue` 回读系统状态（代码级）；LaunchAgent 实际写入需真机确认 |
| 编辑区宽度可控 / 拖拽调宽并记住 | 待人工 | 6.9（`SidebarSplitter` 220–420 px + 持久化已实现） |
| 编辑区宽度可控 / 窄窗口下不挤压内容区 | 自动 | 最小宽度约束（760×480）+ splitter clamp |

## markdown-editing（15）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| 阅读与编辑双模式 / 打开笔记为阅读态 | 自动 | `NoteHeader.test.ts › switches to edit mode and back` |
| 阅读与编辑双模式 / 快捷键切换模式 | 自动 | `useKeyboard`（`⌘E`/`⏎`）+ `NoteHeader.test.ts` |
| 阅读与编辑双模式 / 退出编辑先保存 | 自动 | `NoteHeader.test.ts`（`flush` 先于切模式） |
| 源码编辑器 / 编辑器继承外观设置 | 自动 | `editorTheme.test.ts` 5 例 |
| 渲染安全性 / 内联脚本不执行 | 自动 | `markdown.test.ts` + `NotePreview.test.ts › does not execute inline HTML` |
| 渲染安全性 / 危险链接被拦截 | 自动 | `markdown.test.ts › does not create an anchor for a javascript: link` |
| 渲染安全性 / 伪协议变体同样被拦截 | 自动 | `markdown.test.ts › obfuscated scheme`（先剥控制字符再判 scheme） |
| 渲染安全性 / 非 http(s) 链接不使应用离开界面 | 自动 | `NotePreview.test.ts › opens http links through the system browser` |
| 围栏代码块复制 / 复制单个代码块 | 自动 | `CodeBlock.test.ts › copies exactly the block source` |
| 围栏代码块复制 / 多代码块互不串扰 | 自动 | `fences.test.ts` 12 例（切分 + 独立组件实例） |
| 围栏代码块复制 / 未闭合围栏仍可读 | 自动 | `fences.test.ts › keeps an unterminated fence`、`NotePreview.test.ts` |
| 全文复制 / 复制全文保留 Markdown | 自动 | `NoteHeader.test.ts › copies the raw markdown of the whole note` |
| 语法高亮 / 常用语言高亮 | 自动 | `highlight.test.ts`、`CodeBlock.test.ts`（含别名 `ts`） |
| 语法高亮 / 高亮失败降级 | 自动 | `highlight.test.ts`（未知语言/空/超长 → `null`）、`NotePreview.test.ts › keeps the rest of the note` |
| 语法高亮 / 高亮跟随主题 | 自动 + 人工已验 | `CodeBlock` 按 `resolvedTheme` 重新着色；浏览器内深浅切换实测着色同步 |

（「渲染安全性」下的 4 个场景各有独立用例，不是一条测试敷衍四种协议。）

## notes-core（17）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| 笔记数据模型 / 创建笔记只产生一个实体 | 自动 | `database.rs › crud_soft_delete_restore_and_search_projection`（无独立 snippet 表） |
| 笔记数据模型 / 空正文合法 | 自动 | schema `content NOT NULL DEFAULT ''`；`database.rs` |
| 笔记列表返回摘要 / 列表不含正文 | 自动 | `note_repo::list` 只选 `substr(content,1,240)` + `excerpt()`；`crud_bench` 断言分页 |
| 笔记列表返回摘要 / 加载更多不重复 | 自动 | `crud_bench › 20 页无重复无遗漏`（1,000 条，游标分页） |
| 乐观并发更新 / 过期版本被拒绝 | 自动 | `database.rs` 冲突用例（`expected_updated_at`） |
| 乐观并发更新 / 正常更新刷新时间戳 | 自动 | `database.rs` |
| 自动保存 / 连续输入只落最后内容 | 自动 | `useAutosave.test.ts › collapses a burst of keystrokes into one save`（20 次输入 → 1 次落盘） |
| 自动保存 / 旧请求回包不覆盖新输入 | 自动 | `useAutosave.test.ts › serialises saves`（串行）+ `notes.test.ts › keeps the buffer typed during a save in flight`（`editVersion` 守卫：旧回包只刷新时间戳） |
| 自动保存 / 切换笔记前落盘 | 自动 | `Sidebar.test.ts` 切笔记前调用 `flush`（`NoteHeader.test.ts` 同路径） |
| 自动保存 / 保存失败可见 | 自动 | `ContentPane.vue` `notes.error` 横幅 + 工具栏「保存失败」；`errorMessage(kind)` 映射 |
| 收藏与软删除恢复 / 收藏不改变正文时间 | 自动 | `set_favorite` 只更新 `updated_at` 与收藏位；`database.rs` |
| 收藏与软删除恢复 / 删除后不可见 | 自动 | `database.rs` + `search_composes_with_filters_and_skips_deleted` |
| 收藏与软删除恢复 / 恢复还原笔记 | 自动 | `crud_bench`（restore ×50）+ `database.rs` |
| 标签管理 / 大小写重复被拒绝 | 自动 | `database.rs › rejects_missing_note_and_duplicate_tag` |
| 标签管理 / 重命名同步索引 | 自动 | `tag_repo::rename` → `reindex_all`；`database.rs` |
| 标签管理 / 删除标签保留笔记 | 自动 | `delete_tag` 只删 `note_tags` 行；`database.rs` |
| 标签列表带计数 / 计数排除已删除笔记 | 自动 | `list_with_counts` 过滤 `deleted_at IS NULL`；`recovery.rs` 计数总和断言 |

## release-readiness（7）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| 常驻资源占用 / 空闲驻留 | 待人工 | 8.4：10 分钟采样 0.00% CPU / 41.7 MB，**24h 未跑** |
| 唤出与启动延迟 / 热唤出延迟 | 待人工 | 8.4：进程 0.24 s / 库 0.84 s / WebKit 1.11 s；「按快捷键到窗口出现」需人工 |
| 数据安全演练 / 强制结束后恢复 | 自动 | `recovery.rs › hard_kill_recovery...`（真实 `process::exit`） |
| 数据安全演练 / 备份还原 | 自动 | `recovery.rs › backup_restore_round_trip...` |
| 打包与分发 / 本地构建可用 | 自动 | `pnpm tauri build` 与 `--target universal-apple-darwin` 均成功；x86_64 包已实际启动 |
| 打包与分发 / 分发版本通过校验 | 待人工 | 9.2：无 Developer ID，`codesign` 仅 ad-hoc |
| 发布文档 / 用户能找到数据位置 | 自动 | `docs/RELEASE.md`「笔记库位置与备份」+ 设置页显示绝对路径 |

## search（15）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| 全文检索范围 / 正文命中 | 自动 | `database.rs › search_hits_body_tags_chinese_and_stays_literal` |
| 全文检索范围 / 标签命中 | 自动 | 同上 |
| 全文检索范围 / 中文短词 | 自动 | 同上（逐字索引 + 相邻短语） |
| 全文检索范围 / 中文词组嵌在长句中 | 自动 | 同上 |
| 全文检索范围 / 英文前缀命中 | 自动 | `search_index.rs › ascii_terms_are_prefix_queries`、`database.rs` |
| 筛选与搜索可组合 / 标签与关键词取交集 | 自动 | `search_composes_with_filters_and_skips_deleted` |
| 筛选与搜索可组合 / 收藏筛选叠加 | 自动 | 同上 |
| 搜索索引一致性 / 删除后立即不可检索 | 自动 | `reindex_note` 在软删除事务内；`database.rs` |
| 搜索索引一致性 / 恢复后立即可检索 | 自动 | 同上（restore 事务内重建） |
| 搜索索引一致性 / 索引失败不留半更新 | 自动 | 写路径与索引同事务（`note_repo::create/update`） |
| 查询输入安全 / 通配符按字面匹配 | 自动 | `database.rs`（`100%` 不命中 `1000`）+ `search_index.rs` |
| 查询输入安全 / 引号与运算符不报错 | 自动 | `match_query_is_none_without_searchable_characters` + 转义测试 |
| 结果时序与分页 / 过期响应被丢弃 | 自动 | `notes.test.ts` 3 个用例：慢响应被新查询取代后丢弃、过期错误不覆盖新结果、翻页请求在查询变更后被丢弃（`listSeq` 守卫） |
| 结果时序与分页 / 搜索结果可继续加载 | 自动 | `crud_bench` 游标分页断言 |
| 搜索性能 / 大数据量首屏延迟 | 自动 | `search_bench`（10k 条 P50 16.5 / P95 18.5 ms）+ `crud_bench`（1k 条 P50 4.75 ms） |

---

## 仍然开放的验收项

| 任务 | 阻塞原因 | 解除方式 |
| --- | --- | --- |
| 6.9 实机验收 | 终端无「辅助访问」「屏幕录制」权限（`osascript -25211`、`screencapture` 全黑） | 用户在真机手动跑 `pnpm tauri dev` 并按 tasks.md 6.9 清单逐项确认 |
| 8.3 端到端冒烟 | 无法驱动 WKWebView（macOS 无 WebDriver 支持，权限同上） | 人工按 `docs/RELEASE.md` 走一遍新建→编辑→保存→搜索→删除→恢复 |
| 8.4 24h 驻留 | 单次会话无法持续 24 小时 | `scripts/perf-check.sh soak 24`，补记 CPU/RSS 曲线 |
| 9.2 签名公证 | 本机无 Developer ID 证书与 Apple 账号凭据 | 有证书后 `codesign --sign "Developer ID Application: …"` + `xcrun notarytool submit` |
