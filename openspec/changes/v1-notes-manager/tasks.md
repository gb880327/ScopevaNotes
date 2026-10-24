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
- [ ] 2.10 数据层压测：1,000 条随机笔记的增删改查耗时与列表分页正确性。

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
- [ ] 5.6 路径失效时的界面告知与「重新选择 / 使用默认库」入口（当前仅回退并写日志）。
- [x] 5.7 笔记库统计：笔记数量与文件大小。
- [ ] 5.8 云同步目录提示，以及外部修改/锁冲突时的可见错误。
- [ ] 5.9 迁移/备份失败演练：只读目录、损坏文件、迁移中断、版本过新。

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
- [ ] 7.4 跟随系统即时切换与对比度检查（正文/次要文字/代码块/选中态）。
- [ ] 7.5 CodeMirror 与 Shiki 主题随外观设置联动。

## 8. 质量与可验证性

- [x] 8.1 Rust 单元测试（窗口几何）、集成测试（数据层）、前端单元测试（围栏）。
- [ ] 8.2 组件测试：左栏过滤与滚动、空态、键盘可达性。
- [ ] 8.3 端到端冒烟：新建 → 编辑 → 自动保存 → 搜索 → 删除 → 恢复。
- [ ] 8.4 24 小时驻留的资源与空闲 CPU 测量；冷/热启动耗时记录。
- [ ] 8.5 断电/强杀恢复演练与备份还原往返。
- [ ] 8.6 把本清单中未勾选的验收项与本 change 的规格场景逐条对齐，全部通过后才能勾选。

## 9. 发布（release-readiness）

- [ ] 9.1 发布构建产出可运行 `.app`（Intel 与 Apple Silicon）。
- [ ] 9.2 Developer ID 签名 + 公证流程与校验（对外分发前完成）。
- [ ] 9.3 发布文档：安装、快捷键、库位置与备份、已知限制；补 `CHANGELOG`。
- [ ] 9.4 清理开发残留：移除临时绝对路径、调试输出与未使用依赖。
