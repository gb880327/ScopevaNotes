## 0. 分支与骨架

- [x] 0.1 开分支 `feat/quick-tabs` 再开始任何实现。验证：`git branch --show-current` 输出 `feat/quick-tabs`，`main` 上 `git status --porcelain` 为空。
- [x] 0.2 `migrations/0005_quick_tabs.sql`：只建 `quick_tabs`（id/title/color/content/sort_order/created_at/updated_at）与 `idx_quick_tabs_sort`；`migrations.rs` 只在 `MIGRATIONS` 数组追加一行，runner 逻辑与 `REINDEX_VERSIONS` 不动。验证：删库重启后 `sqlite_master` 有该表与索引，`schema_migrations` 多一条版本 5，现有 4 个迁移文件字节未变（`git diff --stat` 只有新文件）。

## 1. 后端：数据层

- [x] 1.1 `models/quick_tab.rs`：`QuickTab` 序列化为 camelCase（`id/title/color/content/sortOrder/createdAt/updatedAt`）。验证：`cargo build` 通过，`tests/database.rs` 或新文件中一次 JSON 序列化断言字段名。
- [x] 1.2 `repositories/quick_tab_repo.rs`：`list` / `create` / `update` / `delete` / `duplicate` / `count`，全部参数化 SQL、短事务，时间戳 Unix 毫秒、id UUIDv7。验证：新增 `tests/quick_tabs.rs`（`--test-threads=1`）覆盖 CRUD + 复制，断言复制生成新 id 且正文独立。
- [x] 1.3 上限 12 在 Rust 侧强制：`create` 在事务内 `COUNT(*)`，达到上限返回 `AppError::Invalid("速记最多 12 个")`，且不写入任何行。验证：同一测试文件——先建 12 条，第 13 条报 `invalid` 且 `count` 仍为 12；删一条后可再建，回到 12。
- [x] 1.4 隔离断言：速记写入后 `notes`、`notes_fts`、`tags`、`note_tags` 行数不变；`note_repo::list` 与搜索都拿不到该内容。验证：同一测试文件断言四张表行数不变 + 一次 `note_repo::list(search)` 返回空。
- [x] 1.5 正文保真：含换行/制表符/emoji/CRLF 的正文写入后读回逐字节一致。验证：同一测试文件用 `assert_eq!` 对比原文。
- [x] 1.6 `commands/quick_tabs.rs` + `lib.rs` 注册（纯追加，不动现有 27 条）。验证：`cargo build` 通过，`grep` 确认 5 个命令名出现在 `invoke_handler`。

## 2. 后端：窗口与托盘

- [x] 2.1 面板窗口创建：`WebviewUrl::App("quick-tabs.html")`、`label = "quick-tabs"`、`visible(false)`、`decorations(false)`、`always_on_top(true)`、`visible_on_all_workspaces(true)`、480×320、最小 360×220。验证：启动后 `app.get_webview_window("quick-tabs")` 存在且不可见。**已做**：启动即隐藏由真机探针第 1 步确认；尺寸 960×640 是物理像素（scale 2），逻辑尺寸正是 480×320，见 `verification.md`「真机探针结果」。
- [x] 2.2 `on_window_event` 按 `label` 分支：`main` 分支逐字保留，`quick-tabs` 分支 `prevent_close()` + 隐藏面板。验证：面板隐藏、进程存活、**主窗口显隐不变**。**已做**：探针第 3 步在真实进程里 `close()` 面板，`main.visible` 前后都是 `false`；真实鼠标点关闭按钮仍待人工（见 `verification.md` 待人工清单）。
- [x] 2.3 面板显隐状态 + D5 护栏：`last_blur_hide_at`（250 ms 内忽略左键唤醒）、Esc 隐藏、失焦隐藏、显示后聚焦编辑区、记住尺寸不记坐标。验证：四种顺序（点托盘、点桌面、Esc、连点两次托盘）。**已做**：探针在真实进程里跑出 6 步事件顺序（含失焦隐藏、连点两次托盘收敛），记录在 `verification.md`；Esc 只有组件级证据，**250 ms 窗口内的时序仍待人工**。
- [x] 2.4 定位：用 `TrayIconEvent::Click { rect }` 的物理矩形换算锚点，纯几何函数放 `platform/macos/monitor.rs`（或同级新纯函数模块），clamp 进该显示器可见区。验证：`cargo test --lib` 新增 3 条纯函数单测（正常屏、负坐标副屏、图标贴屏幕右缘时不越界）。
- [x] 2.7 全屏 Space：把面板换成运行时声明的非激活 `NSPanel`（`ScopevaQuickTabsPanel`，重写 `canBecomeKeyWindow`）+ `canJoinAllSpaces | fullScreenAuxiliary`，见 design D16。探针在桌面环境确认换类后 `key=true focused=true level=25 behavior=0b100000001`；**同一显示器上的全屏应用已由用户本人验证通过（口头确认，无截图留档）**，全屏里点图标面板直接出现在其上并能打字、不切桌面。**副屏全屏仍待人工**（记为独立待验项）。
- [ ] 2.8 全屏 Space 里贴顶而非挂菜单栏（design D18）：显示后轮询 `NSMenu.menuBarVisible`（300ms × 6），首次消失即把面板上移到屏幕顶边 +2px（水平不动）。验证：桌面态日志 `bar_visible=true -> (516,18)` 且无 snap、`cargo test --lib` 23 passed；**全屏态贴顶效果待用户真机确认**。
- [ ] 2.5 托盘左键改目标，菜单项 `toggle` 的 label 改「显示主窗口」（ID 与行为不变）；右键菜单保持现状。验证：真机左键只切面板、右键只弹菜单、菜单项点到的是主窗口。
- [ ] 2.6 `ui://library-changed`：切库命令成功后追加一次 `emit`（对现有命令是加一行）；面板收到后重新 `list_quick_tabs()` 并按 id 丢弃孤儿草稿。验证：真机切库后面板列出新库的速记，旧草稿不复活。

## 3. 前端：入口与数据

- [x] 3.1 `quick-tabs.html` + `src/quick-tabs.ts` + `vite.config.ts` 的 `rollupOptions.input`（`index.html` / `main.ts` / `App.vue` 零改动）。验证：`pnpm vite build` 产出两个 HTML，`git diff` 显示 `index.html`、`src/main.ts`、`src/App.vue` 未变。
- [x] 3.2 `services/types.ts` 加 `QuickTab` DTO，`services/commands.ts` 末尾追加速记命令段（不在别处新建 IPC 文件）。验证：`pnpm vue-tsc --noEmit` 通过，`grep` 确认无第二个 `invoke(` 调用点出现在 `services/` 之外。
- [x] 3.3 `stores/quickTabs.ts`：`tabs` / `activeTabId` / `loading` / `saveState` / `lastError` / `atLimit`；写路径一律"改本地 → 调命令 → 失败回滚并保留 draft"。验证：新增 `stores/quickTabs.test.ts` 覆盖创建、切换前 flush、删除确认后调用、上限错误透传。

## 4. 前端：面板 UI

- [x] 4.1 `QuickTabsApp.vue` 骨架：顶栏圆点 + `+` + 「更多」、`textarea`、底部状态栏（`正在保存… / 已保存 / 保存失败` + 字符数）。验证：新增 `QuickTabsApp.test.ts` 渲染 3 个速记，断言 3 个圆点与状态栏文本。
- [x] 4.2 圆点交互：点击切换、`+` 新建并切换、选中态用描边/外圈而非仅颜色、每个圆点带可访问名称（颜色 + 标题）。验证：组件测试断言选中元素有 `.active` 且存在 `aria-label`；对比度由既有 `tokens.test.ts` 模式追加一条断言。
- [x] 4.3 更多菜单（in-flow，不用会被裁切的绝对定位）：重命名、换色（固定七色键）、复制、删除；删除永远确认且文案含"不可恢复"。验证：组件测试覆盖三项：取消不发命令、确认调用一次、复制后新条目 id 不同。
- [x] 4.4 自动保存 + 草稿恢复（D7/D10）：复用 `createAutosaveQueue`，300 ms 防抖；`localStorage` 快照，成功后清除，挂载时快照更新则恢复并标未保存。验证：`stores/quickTabs.test.ts` 三条用例——失败保留 draft 且状态为失败、成功清快照、挂载恢复比库新的草稿。
- [x] 4.5 键盘与 IME：Esc 隐藏、方向键与原生 `⌘A/⌘C/⌘V` 不被拦截、组合输入期间不触发界面动作。验证：组件测试断言 Esc 触发关闭回调、非 Esc 键不触发。
- [x] 4.6 主题：复用全局浅/深/跟随系统令牌，文字对比度 ≥ 4.5:1，圆点颜色走固定键映射。验证：`tokens.test.ts` 追加断言，`useTheme.test.ts` 不改。
- [x] 4.7 标签栏布局与视觉（design D17）：整栏 56px、左右两端对齐、全部垂直居中、左侧组间距 10px、圆点点击区 36×36、选中 40×40、`+` 40×40、「更多」高 40px；圆点组超宽横向滚动不换行；hover 用 `--hover`，选中态同时靠尺寸与双环表达。验证：`QuickTabsApp.test.ts`（10 条）与 `tokens.test.ts`（15 条）全部通过；`.active` 断言不变。**真机观感待用户确认**。

## 5. 收口

- [x] 5.1 文档：`CHANGELOG.md` 增条目、`README.md` 功能清单补「速记」、`docs/RELEASE.md` 写用户视角（菜单栏左键呼出、上限 12、无出口、删除不可恢复、升级后勿用旧版本打开新库）。验证：三处存在且与 spec 表述一致。
- [x] 5.2 `verification.md`：需求 → 证据矩阵（自动 / 人工已验 / 待人工），M0 的真机结论（含事件顺序实测）、面板入口 bundle 体积、空闲内存增量。验证：`openspec validate add-quick-tabs --strict` 通过，矩阵无空洞。
- [x] 5.3 全量回归：`pnpm vue-tsc --noEmit`、`pnpm vitest run`、`pnpm vite build`、`cargo test --lib`、5 个集成测试文件（`--test-threads=1`）。验证：全部通过，且现有 169 个前端用例与全部 Rust 用例**断言零修改**（`git diff` 只显示新增用例）。
- [ ] 5.4 隔离回归：确认主窗口行为零变化——⇧⌘D 切主窗口、重复启动唤主窗口、关闭主窗口隐藏、退出确认、单实例、居中定位。验证：真机逐条走一遍并记入 `verification.md`。
- [ ] 5.5 归档时一并删除 `docs/scopeva-notes-quick-tabs-implementation-plan.md`（内容已全部吸收进 `design.md` 与 specs，留着会让 `docs/` 变第二基线）；`docs/` 只保留 `ARCHITECTURE.md`、`RELEASE.md`、logo 与图片。验证：`ls docs/` 不含该文件，`git status --porcelain` 干净。
