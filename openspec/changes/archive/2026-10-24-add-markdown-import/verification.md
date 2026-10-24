# 规格场景 ↔ 验收证据对照（5.5）

`tasks.md` 5.5 要求把 `specs/markdown-import/spec.md` 的 12 条 Requirement / 29 个 Scenario
逐条映射到证据。状态沿用 v1 的约定：

- `自动` — 有可重复执行的测试（给出文件与用例名）。
- `人工已验` — 本机实际观察过。
- `待人工` — 需要真实点击 / 真实 vault 才能确认，当前没有记录。

本次结论：**29 个场景全部有归属**——27 个有自动化测试，**2 个待人工**（菜单栏入口手点、
导入期间真实锁冲突的可见错误）。没有「规格要求但无人负责」的孤儿场景。

自动化证据位置：

- 服务层单测：`src-tauri/src/services/import_service.rs`（`cargo test --lib`，8 个）
- 服务/数据层集成：`src-tauri/tests/import.rs`（`cargo test --test import -- --test-threads=1`，8 个）
- 组件与 store：`src/features/import/ImportDialog.test.ts`（5 个）、`src/stores/notes.test.ts`（导入后刷新）
- 规模：`src-tauri/tests/import_bench.rs`（`--ignored`，1,000 / 5,000 条）

---

## 导入入口与三阶段流程（3）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| 从菜单栏进入导入 | 待人工 | 已接线：`tray.rs` 新增「导入 Markdown…」并 `emit ui://import-markdown`（同时 `window.show`，不 toggle）；`MainWindow.vue` 监听 → `ui.openImport()`；`SettingsDialog.vue` 同一入口。**菜单项未在真实应用里手点过** |
| 导入中不可关闭 | 自动 | `ImportDialog.test.ts › refuses to close while the import is running and offers cancel`（进度阶段无关闭按钮；点遮罩仍停留在模态并提示） |
| 完成后可返回 | 自动 | 同上文件首个用例：报告页关闭后 `ui.importOpen === false`，重新打开回到选择阶段而非旧报告 |

## 目录发现与跳过规则（3）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| 跳过 Obsidian 配置目录 | 自动 | `import_service.rs › discover_skips_hidden_node_modules_and_symlinks`（`.obsidian`/`.trash` 不入列表）；`import.rs › scan_is_read_only_and_matches_execution`（`hidden_skipped == 1` 并出现在预览） |
| 只收 Markdown | 自动 | 同上：`notes.txt`、`pic.png` 不计入 |
| 符号链接不跟随 | 自动 | 同上：`symlink(dir, dir/loop)` 不产生重复遍历，`skipped_hidden` 计入 |

## 预览统计（dry-run）（2）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| 预览不写库 | 自动 | `import.rs › scan_is_read_only_and_matches_execution`：`notes` / `tags` / `notes_fts` 三张表计数在 `scan()` 前后不变，随后执行导入的条数与预览一致（防「预览 300、实导 280」） |
| 预览显示将跳过的已导入项 | 自动 | `import.rs › cancel_keeps_committed_batches_and_retry_fills_the_gap`：第三次跑同一目录 `preview.to_import == 0`、`preview.skipped_existing == 300` |

## 解码与文件级失败隔离（3）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| UTF-8 BOM | 自动 | `import_service.rs › decode_prefers_utf8_and_falls_back_to_gb18030`：`EF BB BF` 前缀被剥离且不计为 GB18030 |
| GB18030 回退 | 自动 | 同上的 GB 分支（`B1 CA BC C7` → 「笔记」，`gb == true`）；`import.rs › undecodable_file_is_reported_and_others_still_import` 断言 `report.decoded_as_gb18030 == 1` 且该笔记成功写入 |
| 无法解码 | 自动 | 同上：`FF FF FF FF` 进入 `failed`（带路径与原因），笔记总数只增加可解码的 4 条，无半条写入 |

## frontmatter 与标题、标签提取（5）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| 剥掉 frontmatter，正文不变 | 自动 | `import_service.rs › body_is_byte_identical_after_frontmatter_strip`（逐字节比较）+ `frontmatter_variants_are_parsed_and_removed` |
| 标题优先级 | 自动 | `import_service.rs › title_priority_and_h1_detection`：FM `title` > 首个 H1（跳过围栏内的 `#`）> 文件名；H1 保留在正文 |
| 复用同名标签 | 自动 | `import.rs › import_batches_reports_progress_and_tags_the_batch`：1,000 个文件都写 `tags: [work]`，`tags` 表中 `work` 仍只有 1 行 |
| 嵌套标签 | 自动 | `frontmatter_variants_are_parsed_and_removed`：`- #nested/tag` → 标签名 `nested/tag`（保留斜杠） |
| 可选的目录名标签 | 自动 | `import.rs › parent_dir_tag_option_adds_one_tag_per_directory`：开启后 `Projects`/`Inbox` 各 2 条，关闭时不存在 |

## 时间戳保留原始顺序（1）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| 旧笔记不挤占列表顶部 | 自动 | `import.rs › scan_is_read_only_and_matches_execution`：导入后 `notes.updated_at` 与文件 `mtime` 毫秒级相等（不是导入时刻）。列表按 `updated_at DESC, id DESC` 排序是 v1 已验契约（`verification.md` 搜索结果与分页） |

## 正文保真与未迁移内容的可见说明（3）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| 图片引用保持原样 | 自动 | `body_is_byte_identical_after_frontmatter_strip`：`![](img/a.png)` 逐字保留；`local_file_refs_count_only_local_files`：只计数本地引用（外链、`#anchor`、`[[笔记]]` 不计） |
| 双链不被改写 | 自动 | 同一 fixture 断言 `[[另一条笔记\|别名]]`、`==高亮==`、`%%注释%%`、`> [!note]`、`^block1` 全部原样 |
| 只读原目录 | 自动 | `import.rs › scan_is_read_only_and_matches_execution`：导入前后对整棵树逐条比较（路径、文件大小、mtime），含 `.obsidian` 与图片 |

## 幂等重试（3）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| 重跑同一目录 | 自动 | `cancel_keeps_committed_batches_and_retry_fills_the_gap`：300 条导入后再跑 → `imported == 0`、`skipped_existing == 300`、总数仍 300 |
| 中途失败后重试 | 自动 | 同一用例的等价路径：取消后保留已提交批次（100 条），重跑只补 200 条并跳过 100 条。磁盘错误路径未单独注入（失败文件隔离另有 `undecodable…` 用例） |
| 手工新建的笔记不受影响 | 自动 | `import_batches_reports_progress_and_tags_the_batch`：手工笔记无来源路径 → 不计入跳过、不被加批次标签、标题/正文/`updated_at` 不变 |

## 批量提交、进度与取消（3）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| 大批量导入可取消 | 自动 | `cancel_keeps_committed_batches…`（300 条，批 100，取消后停在该批边界）；`import_bench` 5,000 条全程 2.12 s、单批最长 131 ms，取消的响应窗口是「一个批次」 |
| 进度可见 | 自动 | `import_batches_reports_progress_and_tags_the_batch`：事件序列 `[200, 400, 600, 800, 1000]`（每批一次）；`ImportDialog.test.ts` 渲染「已导入 X / Y」与进度条 |
| 锁冲突不静默 | 待人工 | 机制复用 v1：`AppError::Busy`（`tests/failures.rs` 映射）+ 自动保存失败保留 dirty（`useAutosave.test.ts`）+ `LibraryNotice`/错误提示。**导入期间真实的锁冲突未演练**；进程内单连接下自动保存只会排队等待（上界 131 ms），`busy` 更可能来自外部进程持锁 |

## 导入报告（2）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| 部分失败 | 自动 | `undecodable_file_is_reported_and_others_still_import`（计数与原因）+ `ImportDialog.test.ts › lists every failure with its reason when some files fail`（逐条渲染路径与原因，失败数徽标） |
| 没有失败时不显示失败清单 | 自动 | `ImportDialog.test.ts` 首个用例：`failed == []` 时 `.failures` 节点不存在 |

## 整批撤销（2）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| 撤销后进入回收站 | 自动 | `import.rs › delete_import_batch_moves_notes_to_trash_and_restores`：`soft_delete_by_tag` → 6 条 `deleted_at` 非空、回收站可 `restore`；`ImportDialog.test.ts › deletes the batch and reports the trash result`（按钮 → `deleteImportBatch(batchTag)` → 文案） |
| 撤销只影响本批 | 自动 | 同上：导入前的「早前的笔记」在撤销后仍存活、重复撤销只影响被恢复的那条 |

## 导入后的可用性（2）

| 场景 | 状态 | 证据 |
| --- | --- | --- |
| 导入的笔记可被搜索 | 自动 | `insert_imported_keeps_timestamps_tags_and_search`（导入与 FTS 同一事务）、`import_batches…`（1,000 条导入后搜索「迁移验证关键字」命中） |
| 既有笔记不变 | 自动 | `import_batches…`：手工笔记标题/正文/`updated_at` 与导入前一致；`note_tags` 中无批次标签 |

---

## 仍然开放的验收项

| 项 | 阻塞原因 | 解除方式 |
| --- | --- | --- |
| 「从菜单栏进入导入」手点 | 需要真实 GUI 点击菜单栏；本机无自动化菜单驱动手段 | `pnpm tauri dev` → 菜单栏图标 → 「导入 Markdown…」→ 看到三阶段模态；再在设置面板走一次同一入口 |
| 导入期间真实锁冲突 | 需要外部进程持写锁与正在编辑的笔记 | 用第二个进程开同一 `.db` 并 `BEGIN IMMEDIATE` 持锁，在应用里编辑并等待自动保存 → 应看到可读的 busy 错误且内容仍为 dirty |
| 真实 Obsidian vault 端到端（5.1） | 需要用户自己的 300 条 vault 与观感判断 | 按 `tasks.md` 5.1 的清单跑一遍，并确认报告里的「未迁移本地文件」计数与实际图片数一致 |

规模实测（`import_bench`，release，20 目录 / 每篇约 600 字中文）：
1,000 条约 0.49 s；5,000 条约 2.12 s（改前 9.52 s，见 design D14），单批最长间隔 131 ms。
