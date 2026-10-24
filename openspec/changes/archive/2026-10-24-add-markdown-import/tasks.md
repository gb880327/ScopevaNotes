## 1. 数据层

- [x] 1.1 新增迁移 `src-tauri/migrations/0004_import_source.sql`：给 `notes` 增加可空列 `source_path`、`content_hash`，并建部分唯一索引 `CREATE UNIQUE INDEX idx_notes_source_path ON notes(source_path) WHERE source_path IS NOT NULL`。验证：`cargo test --test database -- --test-threads=1` 全绿，且旧库升级后既有 50 条笔记的 `source_path` 为 NULL、查询结果不变。
- [x] 1.2 `note_repo` 增加批量导入插入入口（与 `create` 同源：同一事务内 `INSERT` + `replace_tags` + `search_index::reindex_note`），并支持写入 `source_path` / `content_hash` / 指定 `created_at` / `updated_at`。验证：新增集成测试断言插入后 FTS 立刻可命中，且与 `create` 产出的行结构一致。
- [x] 1.3 实现「已导入即跳过」查询：按 `source_path` 判断存在性，供预览统计与执行阶段共用。验证：同一路径连续插入两次只产生一条笔记（唯一索引兜底也覆盖并发路径）。

## 2. 发现与解析（Rust）

- [x] 2.1 `services/import_service.rs` 的目录发现：递归收集 `.md`/`.markdown`，跳过隐藏目录与文件（含 `.obsidian/`、`.trash/`）、`node_modules/`、符号链接，并统计被跳过的隐藏条目数。验证：单元测试用临时目录构造上述结构，断言收集清单与跳过计数。
- [x] 2.2 解码：UTF-8 优先（去 BOM），非法时用 `encoding_rs::GB18030` 回退并标记，仍失败则产出失败项。验证：三个 fixture（UTF-8 with BOM / GB18030 中文 / 非法字节）各一条断言，其中非法字节不产生任何写入。
- [x] 2.3 frontmatter 子集解析：剥离开头 YAML，提取 `title`、`tags`（inline list / 逗号串 / block list，允许 `#` 前缀与 `a/b` 嵌套）、`created`。验证：单元测试覆盖四种写法 + 无 frontmatter + frontmatter 只有未知键（后者正文不含 YAML 且标题回退）。
- [x] 2.4 标题与时间戳规则：`title` → 首个 `# H1` → 文件名（去扩展名）；`mtime` → `updated_at`，`birthtime` → `created_at`，缺失回退，未来时间钳到 `now`。验证：单元测试对同一文件构造不同组合，断言优先级与钳制。
- [x] 2.5 「引用了本地文件」计数：统计正文中指向本地路径的图片/文件引用条数（仅计数，不改写）。验证：单元测试对含 `![](img/a.png)`、`![](https://…)`、`[[嵌入]]` 的样本断言只有本地相对引用被计入。
- [x] 2.6 正文保真断言：解析后正文与原始字节（去掉 frontmatter 段）逐字相等，`[[…]]`、`==…==`、`%%…%%`、`> [!note]`、`^块引用`、`![](img/a.png)` 全部原样。验证：单元测试用一份含全部 Obsidian 语法的 fixture 做字节级比较。

## 3. 命令与执行

- [x] 3.1 `import_scan(path)`：返回预览统计（待导入 / 将跳过 / 隐藏跳过 / 未迁移本地引用），不写库。验证：集成测试断言调用后 `SELECT count(*) FROM notes` 不变。
- [x] 3.2 `import_notes(path, options)`：分批（≤500，默认 200）提交，批间发 `ui://import-progress`、检查取消标志，产出 `ImportReport`。验证：集成测试导入 1000 个生成文件，断言总数、报告字段、以及取消后只保留已提交批次。
- [x] 3.3 取消标志进 `state.rs`（原子布尔），命令启动时复位；取消后报告 `cancelled: true` 且计入已导入条数。验证：`cargo test` 用例在第一批后置位取消，断言不再写入后续批次。
- [x] 3.4 批次标签：导入时创建/复用 `import-YYYY-MM-DD` 标签并挂到本批全部笔记。验证：集成测试断言标签存在、本批笔记均关联、既有笔记未关联。
- [x] 3.5 `delete_import_batch(tag)`（或复用软删除 + 标签筛选）：把该标签下笔记软删除。验证：集成测试断言笔记从 `list_notes` 消失、出现在 `list_deleted_notes`、可 `restore_note` 恢复。
  实际落地：`note_repo::soft_delete_by_tag` 只软删除 `deleted_at IS NULL` 的行并逐条重建 FTS；命令拒绝非 `import-` 前缀的标签（防止一条 `delete_import_batch("work")` 误删整个标签）。
- [x] 3.6 命令注册与错误分类：两个导入命令加入 `lib.rs` 的 `invoke_handler`，参数用 camelCase DTO，失败沿用 `AppError`（含 `Busy` 分类）。验证：`cargo check` 与前端 `vue-tsc` 同步通过，且前端类型与 Rust DTO 字段一致（人工比对 + 类型编译）。

## 4. 前端

- [x] 4.1 `services/commands.ts` 增加 `importScan` / `importNotes` / `deleteImportBatch` 与对应 TS 类型（`ImportPreview`、`ImportReport`、`ImportOptions`）。验证：`pnpm vue-tsc --noEmit` 通过，且类型字段与 Rust 侧一一对应。
- [x] 4.2 `src/features/import/ImportDialog.vue`：三阶段模态（选目录与预览 → 进度 → 报告），复用既有 dialog 样式与令牌，导入中禁止关闭。验证：组件测试覆盖三阶段切换、导入中关闭被拒、取消按钮在进度阶段可见。
- [x] 4.3 目录选择复用 `@tauri-apps/plugin-dialog` 的目录模式；选项区含「用父目录名作为标签」。验证：组件测试 mock dialog 返回值后预览被请求，选项开关进入命令参数。
- [x] 4.4 报告面板：成功 / 跳过 / 失败（逐条路径与原因）/ 隐藏跳过 / GB18030 条数 / 未迁移本地引用 / 耗时，失败清单为空时不渲染该区块，含「删除本批」入口。验证：组件测试对「有失败」与「无失败」两份报告快照断言区块存在性。
- [ ] 4.5 入口接线：`tray.rs` 菜单项「导入 Markdown…」emit `ui://import-markdown`，`MainWindow.vue` 监听后打开模态；设置面板增加同一入口。验证：`pnpm vitest run` 覆盖监听分支，`pnpm tauri dev` 手点菜单项能看到模态。
  代码已完成（菜单项 + `window.show` 而非 toggle + 前端监听 + 设置入口；模态本身有 5 个组件测试）。
  **未勾选的原因**：菜单项尚未在真实应用里手点过，而本任务把「手点能看到模态」写进了验收条件。
- [x] 4.6 导入结束后刷新笔记列表（走 `notes.refresh()`，与 `listSeq` 守卫兼容）。验证：`stores/notes.test.ts` 新增用例断言导入完成后列表重新请求且旧响应被丢弃。

## 5. 验证与文档

- [ ] 5.1 端到端手工验证（待真实 vault）：用一个含 300 条笔记、`.obsidian/`、`.trash/`、frontmatter、双链、图片引用的真实 Obsidian vault 走完全流程，核对报告数字与 `SELECT` 结果一致。验证：报告数字与 SQL 计数逐项相等，截图或记录留在 `verification.md`。
- [x] 5.2 幂等与重试验证：同一目录连续导入两次，第二次「导入 0 / 跳过 300」，笔记总数不变；人为中断后重跑只补齐剩余。验证：两次报告数字与总数对比。
- [x] 5.3 性能验证：1000 条与 5000 条目录各跑一次，记录耗时、批次数与导入后库体积，确认 UI 全程可响应且自动保存在冲突时给出可见错误。验证：记录耗时与 `perf-check` 风格的数字，写入 `verification.md`。
  实测（release，`cargo test --release --test import_bench -- --ignored --nocapture`，20 目录 / 每篇约 600 字中文）：
  1,000 条 0.49 s / 5 批 / 库 2.2 MB；5,000 条 2.12 s / 25 批 / 库 10.8 MB；**单批最长间隔 131 ms**。
  这一项顺带发现并修掉了 `notes_fts` 的 O(n²) 写入（`DELETE` 打在 UNINDEXED 列上），
  5,000 条从 9.52 s 降到 2.12 s（design D14）。
  边界：**「UI 全程可响应」没有实机观察**，只有「自动保存最坏等待 ≤ 单批间隔」这一上界；
  真实锁冲突需要外部进程持锁，见 `verification.md` 的开放项。
- [x] 5.4 文档：`docs/RELEASE.md` 增加导入使用说明与两条限制（不迁移附件、不解析 Obsidian 扩展语法）；`CHANGELOG.md` 增加 0.2.0 段落；`README.md` 阶段清单补一行。验证：三处文档存在且与 spec 的限制表述一致。
- [x] 5.5 规格对照：把 `specs/markdown-import/spec.md` 的 12 条 requirement 逐条映射到测试或手工证据，写进 `verification.md`。验证：`openspec validate add-markdown-import --strict` 通过，且矩阵中无「待人工」以外的空洞。
