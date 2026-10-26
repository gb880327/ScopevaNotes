## Context

See `proposal.md` — Why。现状约束（都已核对代码）：

- 只有一个窗口：`tauri.conf.json` 声明 `label: "main"`，`WindowManager` 的 `LABEL` 硬编码 `"main"`，全部显隐逻辑（gate 互斥、尺寸记忆、居中定位、置顶）只服务这一个窗口。
- `lib.rs` 的 `on_window_event` **不区分窗口**：任何窗口的 `CloseRequested` 都会 `prevent_close()` 并隐藏主窗口。
- 托盘已经是"左键发事件、右键弹菜单"：`tray.rs` 设了 `show_menu_on_left_click(false)`，`on_tray_icon_event` 只匹配 `MouseButton::Left` + `Up`；`tray-icon 0.25.1` 的 macOS 实现里 `menuDown:` / `rightMouseDown:` 是两套独立 responder，右键菜单是默认行为。
- `TrayIconEvent::Click` 带 `rect: Rect`（tauri 2.12.5），物理像素，足以做菜单栏锚点定位，不需要 AppKit。
- 前端单入口：`index.html` → `src/main.ts` → `App.vue` → `MainWindow.vue`；`services/commands.ts` 是唯一 IPC 入口（D11）。
- 自动保存已有可复用件 `composables/useAutosave.ts`（`createAutosaveQueue(run, delay)`，带 `schedule/flush/discard/isDirty`）。
- 退出协议：`ExitRequested` → 广播 `ui://quit-requested` → 主窗口 flush 后调 `request_quit()` → 原生确认框 → `allow_exit()`。
- capabilities 目前 `"windows": ["main"]`。
- 迁移序列 `0001`–`0004`，`REINDEX_VERSIONS = &[3]`。
- 库路径可被用户切换：每条命令都经 `state.db()` 重新取 `Arc<Database>`。

## Goals / Non-Goals

**Goals:**

- 速记与笔记在数据、索引、UI、心智模型四个层面都完全隔离，且隔离是可测的（FTS 无行、回收站条数不变、列表搜不到）。
- 面板的显隐、定位、焦点行为在真实 macOS 上可解释：只有一处"顺序相关"的风险（失焦隐藏 vs 托盘点击），且用时间戳护栏消除。
- 一个字节都不改主窗口的入口文件与既有行为；新增代码尽量落新文件。
- 保存路径在"进程被杀"这类最坏情况下最多丢一个防抖窗口，且能恢复。

**Non-Goals:**

- 不追求 `NSPanel` 的非激活浮窗语义（不引入 `objc2` / unsafe）。
- 不做速记的重排（拖拽圆点）、不做速记搜索、不做速记上限之外的分页或溢出菜单。
- 不做"速记 → 笔记"的任何通道（连隐藏命令也不留）。
- 不重构 `WindowManager` 让主窗口与面板共用一个状态机——那会把风险转移给已经稳定的主窗口。

## Decisions

**D1 速记是独立表，不给 `notes` 加 `kind` 列。**
理由：`notes` 上挂着 FTS 触发器、软删除、标签、导入 `source_path` 唯一索引、回收站计数——加 `kind` 就要在每个现有查询里追加过滤条件，等于把"零改动"变成"遍地改动"。独立表让隔离在 schema 层就成立，也让"速记 MUST NOT 出现在列表/搜索"这条契约变成结构性事实而不是记忆负担。备选（`notes.kind` + 视图）被否：收益为零，风险散布全仓。

**D2 面板是普通 `WebviewWindow`，不是 `NSPanel`。**
配置面：`label = "quick-tabs"`、`visible(false)`、`decorations(false)`、`always_on_top(true)`、`visible_on_all_workspaces(true)`、`min 360×220`、初始 480×320。理由：`visible_on_all_workspaces` 解决"点开面板把用户拽回主 Space"这个最刺眼的问题；非激活浮窗要靠 `objc2` 改窗口类，风险（焦点、IME、⌘V 全可能变脆）与收益不成比例。代价：点开面板会激活 Notes 进程（Dock 图标跳动、系统菜单栏切成 Notes 的菜单）——与"菜单栏常驻但仍是普通 app"的现状一致，接受。**D16 后来推翻了其中的"不追 nonactivating"**：真机上普通窗口进不了别人的全屏 Space，证据与范围写在那里。

**D3 前端用多页入口，不改主窗口入口三件套。**
新增 `quick-tabs.html` + `src/quick-tabs.ts` + `src/windows/quick-tabs/`，`vite.config.ts` 加 `build.rollupOptions.input`；Rust 侧 `WebviewUrl::App("quick-tabs.html")`（dev 下自动解析成 `http://localhost:1420/quick-tabs.html`）。备选"单入口按 `getCurrentWindow().label` 分派根组件"被否：要改 `main.ts` 或 `App.vue`，违背"主窗口文件零改动"这条本轮硬约束。

**D4 托盘左键 = 面板 toggle，菜单项 ID 保持 `toggle`、文案改「显示主窗口」。**
理由：`tray.rs` 的左键通道现状已经正确（区分左右键、右键弹菜单默认成立），只需换目标；菜单项复用同一 ID 与同一业务路径（`state.window.toggle`），避免出现第二条"显示主窗口"的实现。文案必须改，因为左键不再切它，留着「显示 / 隐藏窗口」会骗人。

**D5 失焦隐藏 + 250 ms 护栏，不依赖事件顺序。**
面板可见时点菜单栏图标，事件顺序可能是 `blur→hide` 再 `click→toggle→show`（点了没关掉），也可能是 `click→toggle→hide` 再 `blur`（正常关）。处理：记录 `last_blur_hide_at`，左键到来时若"刚刚因失焦隐藏过（< 250 ms）"则直接忽略。备选"复杂状态机 / 忽略刚显示后的失焦"被否：前者过度设计，后者解决不了这个具体顺序问题。

**D6 定位用托盘 `rect` 的物理坐标 + 纯几何函数。**
`Click { rect }` 给出图标在屏幕上的物理矩形；面板落点为"水平居中对齐图标、垂直贴在图标下方"，再 `clamp` 进该显示器可见区（`monitor.rs` 已有 `clamp_to_area` / `centered_origin` 的可测模式，新增函数同样只做纯计算 + 单测）。不保存坐标（与主窗口的既有规则一致），只保存尺寸。

**D7 草稿双层：数据库 300 ms 防抖 + `localStorage` 快照。**
内存 draft 是唯一真相；每次防抖重置把快照写 `localStorage`，写库成功即删快照，挂载时"快照比库新就恢复"。理由：退出/崩溃路径再怎样都有 ≤300 ms 窗口，而速记的价值恰恰是"随手写、不审视"，丢字比多写一次 localStorage 更伤。备选"纯内存 draft"（方案原文）被否：`ExitRequested` 与面板 flush 之间没有回执机制，无法保证顺序；备选"跨窗口 flush 闸门（Rust 侧 emit + 等回执）"被否：为一句话的收益引入跨窗口协议。

**D8 上限 12 只由 Rust 强制。**
`create_quick_tab` 在事务内 `COUNT(*)`，≥12 返回 `AppError::Invalid("速记最多 12 个")`；UI 的禁用与提示是体验层，不是防线。理由：并发/异常路径下 UI 状态不可信；错误必须可读（不能冒出 SQLite 原文）。

**D9 不新增错误 kind。**
现有 `AppError` 的 `not_found / invalid / busy / database` 覆盖全部速记失败场景（上限与非法重排都是 `invalid`，缺行是 `not_found`，锁冲突由既有 `classify()` 提升为 `busy`）。备选"按方案新增 6 个 kind"被否：会让错误分类体系分叉，前端 `CommandError.kind` 分支翻倍而无对应用户价值。

**D10 复用 `createAutosaveQueue`，面板自己持有一个实例。**
不新建 debounce 实现，也不为每个 Tab 建定时器（方案已要求）。切换 Tab / 隐藏 / 失焦 / 收到 `ui://quit-requested` 时 `flush()`；`discard()` 只在丢弃草稿这种显式场景用。

**D11 `on_window_event` 按 `label` 分支，主窗口分支逐字保留。**
现状是"任何窗口 `CloseRequested` → 隐藏主窗口"，两窗口后必须改成：`main` → 原逻辑；`quick-tabs` → `prevent_close()` + 隐藏面板 + flush。这是本轮唯一必须动主窗口生命周期代码的地方，改动限定在这一个 `match` 里。

**D12 退出：面板只 listen、只 flush、绝不弹框。**
`ui://quit-requested` 是广播，两个 webview 都会收到；面板收到后只 `flush()`，不调 `request_quit()`（否则 `begin_quit_prompt` 之外还会多一次 flush 竞争）。主窗口流程完全不变。

**D13 切库后停靠。**
切库命令成功后由 Rust 侧追加一次 `emit("ui://library-changed")`（对现有命令是加一行），面板收到后重新 `list_quick_tabs()` 并按 D7 的规则丢弃孤儿草稿（id 是 UUIDv7，跨库不会撞）。理由：不这么做，面板会把草稿静默写进刚切换的新库。

**D14 面板包体约束：禁止 import 主窗口特性。**
面板只用 `textarea` + `services/commands.ts` + 主题令牌；MUST NOT import `features/notes/**`、`markdown-it`、`shiki`、CodeMirror。M4 记录面板入口产物体积（预期几 KB），若接近主 chunk 量级说明有违规依赖，必须拆掉。

**D15 实现前先开分支 `feat/quick-tabs`。**
理由：速记要动 `lib.rs`、`tray.rs`、`vite.config.ts` 三个既有文件，即使每处都很小，也不该让它们在 `main` 上处于半成品状态。全部实现与测试在分支完成，`main` 在合并前保持现状；合并前跑全量回归。

**D16 全屏 Space：把面板换成非激活 `NSPanel`（推翻 D2 的"不追 nonactivating"）。**
真机上试错两次，两次都不成立，记录下来免得后人来一遍：

1. **只有 `canJoinAllSpaces`**（也就是 `visible_on_all_workspaces(true)` 的全部效果）：普通桌面之间跟随成立，但进不了*别人*的全屏 Space。macOS 的化解方式是把窗口放回它自己的 Space，用户看到的是"全屏里点图标，面板跑去另一块屏"。
2. **补上 `fullScreenAuxiliary` 但仍是 tao 的普通窗口**：全屏里点图标**完全没反应**。普通窗口在别人的全屏 Space 里拿不到 key，`makeKeyAndOrderFront` 静默失败；此时激活 app 也只会把人拽走。

最终修法（三项缺一不可）：

- 运行时用 `define_class!` 声明 `NSPanel` 子类 `ScopevaQuickTabsPanel`，**重写 `canBecomeKeyWindow → true`**：`decorations(false)` 的无边框窗口默认回答 `false`，而换类会丢掉 tao 自己那个重写——这是第一版换类后 `key=false`、面板能显示但打不进字的直接原因。
- `object_setClass` 就地换类（`AnyObject::set_class` 断言两类 instanceSize 相等，而 `NSPanel` 报 456、`NSWindow` 报 464；换成更小的类不需要额外存储，故走 raw runtime 调用）。
- 面板语义补齐：`styleMask |= NonactivatingPanel`、`floatingPanel = true`、`becomesKeyOnlyIfNeeded = false`、`hidesOnDeactivate = false`、`level = NSStatusWindowLevel(25)`、`collectionBehavior = canJoinAllSpaces | fullScreenAuxiliary`。

探针证据（桌面环境，非全屏；探针在收尾时已删净）：换类后 `class=ScopevaQuickTabsPanel key=true focused=true level=25 behavior=0b100000001`；仅换类不重写 `canBecomeKeyWindow` → `canBecomeKey=false key=false`；完全不换类（对照）→ `canBecomeKey=true key=true level=5 behavior=0b1`。
残余权衡：这是本轮唯一一处 `unsafe` 的窗口类手术，`NSPanel` 的焦点/IME 行为比普通窗口更需要真机确认（见 verification 的待人工清单）。

**D17 标签栏 = 单行 56px，尺寸进 spec。**
用户给的硬指标：整栏 56px、左右两端对齐、全部垂直居中、左侧组间距 10px、圆点点击区 36×36、选中态 40×40、`+` 40×40、「更多」高 40px。实现取舍：

- **不换行、改横向滚动**：12 个圆点 × 36px + 间距已经超过 480px 的面板宽度，`flex-wrap` 会把标签栏撑成两行、把编辑区挤矮，所以 `.dots` 走 `overflow-x: auto` + 隐藏滚动条。
- **选中态 = 尺寸 + 描边**：40px 容器变大给出"当前位置"的位移感，圆点外圈加 2px 背景色 + 1px 文字色的双环，保证不只是颜色差异（spec 的可访问性要求）。
- **hover 只用 `--hover` 半透明层**，不动文字色差，避免和对比度测试打架。
- 圆角统一 `--radius`，`+`/`更多` 用透明底 + `--border-strong` 描边，和主窗口的次要按钮一致。

**D18 全屏 Space 里改为贴顶：显示后跟着菜单栏的消失一起上移。**
全屏时菜单栏自动隐藏：用户把鼠标顶上去唤出、点图标、面板拿到焦点，菜单栏随即滑走——此时"图标下方 6px"就变成"离屏幕顶部一整条菜单栏那么高的空隙"，看着像浮在半空。用户在真机上看到的就是这个。

试过的判据与结论：
- `NSApplication.currentSystemPresentationOptions()`（`AutoHideMenuBar` / `FullScreen`）——真机全屏点图标时仍返回 `false`，**别人的全屏 Space 不反映在这个值里**，弃用。
- `NSScreen.visibleFrame` 的顶部内缩——全屏时仍报 25（等同桌面），也无区分度，弃用。
- 显示**之后**轮询 `NSMenu.menuBarVisible`（类方法，公共 API）——这是唯一能区分"菜单栏此刻是否真的在屏幕上"的信号。

最终做法（两道保险，都不需要知道"现在是不是全屏"）：

1. **把间距从 6px 收到 2px**（`TRAY_GAP = 2.0`）：菜单栏在的时候面板与它几乎齐平（桌面实测顶部 26 逻辑像素、菜单栏下沿 25），"没靠菜单栏"那半截抱怨直接消失；菜单栏不在的时候少 4px 也谈不上"一段间隙"。
2. **显示后跟着菜单栏一起上移**：起后台线程每 400ms 在主线程问一次 `NSMenu.menuBarVisible`，最多 25 次（约 10 秒）；第一次真的消失就把面板上移到屏幕顶边（`FULLSCREEN_TOP_INSET = 2px`，水平不动）。桌面态菜单栏一直在，轮询到上限自然结束，行为零变化。

诚实记录：**全屏那一次的日志始终没采到**（用户几次复现都落在"单屏 + 无全屏"的状态，日志里 `bar_visible` 全程 `true`），所以第 2 条保险的触发**未经真机验证**，只验过它在桌面态不误触发；第 1 条是确定性几何改动，已用单元测试与桌面实测覆盖。

- 面板不可见、位置已经在上方、或 `run_on_main_thread` 失败都会提前退出，不会泄漏线程。

## Risks / Trade-offs

- **失焦隐藏与托盘点击的事件顺序在真机可能与预期不同** → D5 的 250 ms 护栏对两种顺序都收敛到"点击=关掉"，M0 真机确认（这是 M0 唯一必须动手的项）。
- **激活进程带来的副作用**（Dock 图标跳动、系统菜单栏切成 Notes）→ 接受并写进 `docs/RELEASE.md` 的用户说明；若要消除只能上 `NSPanel`/`Accessory` 策略，代价见 D2。
- **多一个常驻 webview 的内存**（估计 +40–80 MB，现有空闲 41.7 MB）→ M4 量一次并记录数字，不设门槛；若不可接受再改成"首次点击懒创建"。
- **`localStorage` 同步写**：1 MB 文本每 300 ms 一次同步写会伤输入流畅度 → 面板内容预期很小；M4 用 1 MB 文本量一次，若 >16 ms 再降级为写 `IndexedDB` 或用 `requestIdleCallback` 合并。
- **`on_window_event` 分支写错会误伤主窗口** → 任务里单列一条回归用例断言"关闭面板不隐藏主窗口、不结束进程"。
- **`0005` 会让 schema 版本升到 5**，旧程序打开新库会被既有校验拒绝（`library` spec 要求"版本不高于程序支持"）→ 与既有语义一致，发布说明里写一句"升级后不要用旧版本打开同一个库"。
- **隔离条款是可逆性成本**：以后若要"转换为笔记"，得先改 `notes-core` 与 `quick-tabs` 两份 spec，而不是随手加个命令 → 接受，这正是契约的意义。

## Migration Plan

1. 开分支 `feat/quick-tabs`（`git switch -c`），实现与测试全部落在分支上。
2. 新增 `migrations/0005_quick_tabs.sql`：只 `CREATE TABLE quick_tabs` + `CREATE INDEX idx_quick_tabs_sort`。`migrations.rs` 的 runner 与 `REINDEX_VERSIONS` 不动。
3. 回滚：分支未合入即 `main` 无变化；已合入后回滚 = 回退提交（旧库里的 `quick_tabs` 表成为无害残留，程序不再读写它）。
4. 合并门槛：全量前端 + Rust 测试绿、`vue-tsc` 干净、`vite build` 通过、`openspec validate --all --strict` 通过、M0 真机项有结论（通过或明确标 `待人工`）。
5. 归档时一并删除 `docs/scopeva-notes-quick-tabs-implementation-plan.md`（其内容已被本文件与 specs 完整吸收）。理由：`docs/` 不能成为第二基线——阶段三刚清空过一次；`openspec/specs/` 归档后是唯一生效基线，`docs/` 只留 `ARCHITECTURE.md`、`RELEASE.md` 与 logo。

## Open Questions

- 面板默认尺寸 480×320 是否需要调整：等 M2 在真机上用人眼看过再定，纯视觉参数，不影响契约。
- 速记重排（拖拽/左移右移）以后是否补：`sort_order` 列已留，补的时候只加一个命令与一段 UI，不改 schema。
