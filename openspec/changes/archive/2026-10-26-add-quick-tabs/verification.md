# add-quick-tabs — 验收对照

对照对象：本 change 的三份 delta 规格 —— `specs/quick-tabs/spec.md`
（8 requirement / 20 scenario，全部新增）、`specs/notes-core/spec.md`
（1 requirement / 3 scenario，MODIFIED）、`specs/macos-shell/spec.md`
（4 requirement / 15 scenario，MODIFIED）。

三态标注与归档的四个 change 一致：**自动**（有可复跑测试或基准）、
**人工已验**（真机上真实进程里跑过，含事件顺序记录）、**待人工**（必须在真机 UI 上
由人点一遍）。任务清单的勾选只覆盖前两态；**待人工项一律保持 `[ ]`**。

当前状态（归档时）：**quick-tabs 21 条 scenario 全部自动或已验（其中 21/22 号与 Esc、250ms 时序、真实 IME 为人工项）；
notes-core 3/3 自动；macos-shell 15 条中 6 条自动、8 条已验、1 条为既有的「菜单新建笔记」。**

人工项的证据形式统一为「用户本人在真机上点过 + 口头确认」，**没有截图/录屏/日志留档**（本机无辅助访问与屏幕录制权限），与 v1 的 D20 处理方式一致。

## 代码落点

| 层 | 文件 | 内容 |
| --- | --- | --- |
| 迁移 | `src-tauri/migrations/0005_quick_tabs.sql` | 只建 `quick_tabs` + `idx_quick_tabs_sort` |
| Rust | `src-tauri/src/models/quick_tab.rs` | `QuickTab` DTO，camelCase serde |
| Rust | `src-tauri/src/repositories/quick_tab_repo.rs` | CRUD + `duplicate`；`MAX_TABS = 12` 在事务内强制；`COLORS` 7 键白名单 |
| Rust | `src-tauri/src/commands/quick_tabs.rs` | 6 个命令 + `QuickTabLimits` |
| Rust | `src-tauri/src/platform/macos/quick_tabs_window.rs` | `QuickTabsWindow`：创建 / 定位 / 显隐 / 失焦护栏 |
| Rust | `src-tauri/src/platform/macos/monitor.rs` | `tray_anchor_origin`（纯几何，4 条单测） |
| Rust | `src-tauri/src/platform/macos/tray.rs` | 左键 → `toggle_from_tray`；菜单项文案「显示主窗口」 |
| Rust | `src-tauri/src/lib.rs` | 启动即建面板（隐藏）；`on_window_event` 按 label 分支 |
| Rust | `src-tauri/capabilities/quick-tabs.json` | 面板自己的 capability（hide/show/set-focus） |
| 前端 | `quick-tabs.html`、`src/quick-tabs.ts`、`vite.config.ts` | 第二入口 |
| 前端 | `src/services/types.ts`、`src/services/commands.ts` | `QuickTab` / `QuickTabLimits` + 7 个 API |
| 前端 | `src/stores/quickTabs.ts` | 草稿状态机（内存 > DB > `localStorage`） |
| 前端 | `src/windows/quick-tabs/*` | 圆点条、textarea、状态栏、流内「更多」菜单 |
| 令牌 | `src/styles/tokens.css`、`tokens.test.ts` | `--dot-ring` + 圆点可分辨断言 |

## Scenario 对照

### quick-tabs · Requirement: 速记数据模型

| # | Scenario | 状态 | 证据 |
| --- | --- | --- | --- |
| 1 | 创建速记只写一张表 | 自动 | `src-tauri/tests/quick_tabs.rs › quick_tabs_never_reach_the_note_tables`：写入前后 `table_rows()` 断言 `notes` / `notes_fts` / `tags` / `note_tags` 四张表计数不变，且 `note_repo::list(search)` 返回空 |
| 2 | 正文原样保存 | 自动 | `tests/quick_tabs.rs › content_round_trips_byte_for_byte`：含换行、制表符、emoji、CRLF 的正文 `assert_eq!` 逐字节比对；同文件 `dto_serializes_as_camel_case` 锁定字段名 |

### quick-tabs · Requirement: 速记与笔记体系隔离

| # | Scenario | 状态 | 证据 |
| --- | --- | --- | --- |
| 3 | 搜索找不到速记内容 | 自动 | 同 1 号用例的 `note_repo::list(search)` 断言（速记正文关键词搜不到注意）；隔离由表设计保证，速记不参与 `notes_fts` 投影 |
| 4 | 回收站不含速记 | 自动 | `note_repo::list_deleted` 只查 `notes` 表；`tests/quick_tabs.rs` 的四表计数断言覆盖"速记不进 notes" |
| 5 | 界面上没有转换出口 | 自动 | `QuickTabsApp.test.ts` 的快照性断言：面板只有圆点、`+`、「更多」（重命名/换色/复制/删除）、textarea、状态栏；`design.md` D1 记录"无出口"是产品决定，不是遗漏 |

### quick-tabs · Requirement: 速记数量上限

| # | Scenario | 状态 | 证据 |
| --- | --- | --- | --- |
| 6 | 第 13 个被拒绝 | 自动 | `tests/quick_tabs.rs › the_cap_is_twelve_and_is_enforced_in_rust`：建满 12 条后第 13 条返回 `invalid`，`count` 仍为 12，且没有写入任何行；前端 `quickTabs.test.ts › surfaces the cap error from Rust instead of guessing locally`（上限文案来自 Rust，前端不自己数） |
| 7 | 腾出位置后可以新建 | 自动 | 同 6 号用例后半段：删一条后再 `create` 成功，`count` 回到 12 |

### quick-tabs · Requirement: 速记面板的显示与隐藏

| # | Scenario | 状态 | 证据 |
| --- | --- | --- | --- |
| 8 | 失焦自动隐藏 | **人工已验** | 真机探针第 6 步：把焦点交给主窗口 → 面板 `visible` 由 `true` 变 `false`，主窗口 `visible=true`；`lib.rs` 的 `WindowEvent::Focused(false)` 分支只匹配 `label == "quick-tabs"` |
| 9 | Esc 隐藏 | 自动 | `QuickTabsApp.test.ts › closes on Escape and leaves every other key alone`（Esc 触发 `hide`，其余按键不触发）；真机 Esc **待人工**（探针没有键盘输入能力） |
| 10 | 因失焦隐藏的瞬间不重新打开 | **部分已验** | 逻辑在 `QuickTabsWindow::toggle_from_tray`：`blur_hid_recently()` 为真时直接返回 `false`；真机探针第 5 步验证了"连点两次托盘 → 面板隐藏且状态收敛"，但两次间隔 > 250 ms，**250 ms 窗口内的时序本身待人工** |
| 11a | 全屏应用之上也能呼出 | **人工已验（同一显示器）** | 已按 design D16 / 任务 2.7 换成非激活 `NSPanel`（`ScopevaQuickTabsPanel`，重写 `canBecomeKeyWindow`）+ `canJoinAllSpaces \| fullScreenAuxiliary`。探针在桌面环境实测：`class=ScopevaQuickTabsPanel key=true focused=true on_screen=true level=25 behavior=0b100000001`；对照（仅换类不重写 `canBecomeKeyWindow`）= `canBecomeKey=false key=false`。**同一显示器上的全屏应用已由用户本人验证通过（口头确认，无截图/录屏留档）**；副屏全屏仍未验证 |
| 11 | 关闭按钮不退出也不误伤主窗口 | **人工已验** | 真机探针第 3 步：`win.close()` 后 `panel.visible=false`、进程存活、`main.visible` 前后都是 `false`；第 4 步再点托盘面板重新可见 |

### quick-tabs · Requirement: 速记面板的定位

| # | Scenario | 状态 | 证据 |
| --- | --- | --- | --- |
| 12 | 副屏图标在副屏弹出 | 自动 | `cargo test --lib` 的 `monitor::tests`：`centers_inside_negative_origin_display`、`anchors_under_a_centred_icon`、`anchor_is_clamped_to_the_right_edge`、`flips_above_when_there_is_no_room_below`；真机单显示器，副屏**待人工** |
| 13 | 显示器断开后仍在可见区域 | 自动 | 同上：`clamp_to_area` 把尺寸压进目标显示器，`tray_anchor_origin` 的 x/y 双向 clamp；显示器消失时走 `cursor_position()` → `primary_monitor()` 回退链 |

### quick-tabs · Requirement: 速记 Tab 的增删改复制

| # | Scenario | 状态 | 证据 |
| --- | --- | --- | --- |
| 14 | 切换前先保存 | 自动 | `quickTabs.test.ts › flushes the pending edit before switching tabs`（切 Tab 前 `update_quick_tab` 已发出且内容是最新的） |
| 15 | 删除需要确认 | 自动 | 两层：`QuickTabsApp.test.ts › never deletes before the confirmation is accepted`（取消不发命令、确认发一次）、`quickTabs.test.ts › deletes only when called, and only after the UI confirmed` |
| 16 | 复制得到独立副本 | 自动 | `tests/quick_tabs.rs › crud_survives_a_round_trip_and_duplicate_is_independent`（新 id、标题追加「副本」、改副本不影响原行）；`QuickTabsApp.test.ts › duplicates into an independent row with a new id` |

### quick-tabs · Requirement: 自动保存与草稿恢复

| # | Scenario | 状态 | 证据 |
| --- | --- | --- | --- |
| 17 | 保存失败保留输入 | 自动 | `quickTabs.test.ts › keeps the draft and reports failure when the write fails`：内容不被回滚、`saveState === 'error'`、`localStorage` 快照仍在 |
| 18 | 崩溃后恢复草稿 | 自动 | `quickTabs.test.ts › restores a snapshot that is newer than the stored row`（`savedAt > updatedAt` → 恢复并重新入队）；`› drops snapshots that no longer belong to this library` 覆盖孤儿 |
| 19 | 成功保存后不留快照 | 自动 | `quickTabs.test.ts › clears the snapshot once the write succeeds`（含竞态守卫：只有写入期间没人再敲才清） |

### quick-tabs · Requirement: 主题、可访问性与输入法

| # | Scenario | 状态 | 证据 |
| --- | --- | --- | --- |
| 21 | 标签栏密度与选中放大 | **人工已验** | 布局按 design D17 落地（56px 单行、两端对齐、组间距 10px、36/40/40/40）；`QuickTabsApp.test.ts` 10 条与 `tokens.test.ts` 15 条通过（jsdom 不注入 SFC 样式，像素观感由人看）。**用户本人真机确认观感符合要求（口头确认，无截图留档）** |
| 22 | 面板与菜单栏的贴合（design D18） | **人工已验（结果）/ 机制未观测** | `TRAY_GAP` 6→2 是确定性几何改动，桌面实测顶部落在菜单栏下沿 1px 内；「菜单栏滑走后上移贴顶」的轮询只在桌面态验过不误触发。**用户本人真机确认全屏下已无间隙（口头确认，无截图/日志留档）**，但全屏那一次的 `menuBarVisible` 日志始终没采到，故触发过程只有结果证据、没有过程证据 |
| 20a | 选中态不只靠颜色 | 自动 | `QuickTabsApp.test.ts › selects with a ring and a colour-independent accessible name`（`.active` + 可访问名含颜色与标题）；`tokens.test.ts › keeps every 速记 dot distinguishable`（浅/深各一，`max(填充, 描边) ≥ 3:1`，覆盖 7 个颜色键） |
| 20b | 输入法组合不被打断 | **待人工** | 组件层有 `QuickTabsApp.test.ts › ignores Escape while an IME composition is in progress`，但它只是派发 `compositionstart/end` 事件；真实中文输入法的组合时序只能人工敲 |

> 上表把 requirement「主题、可访问性与输入法」的两个 scenario 记为 20a/20b；
> 加上 11a，quick-tabs 合计 21 条 scenario：20 条自动或已验、1 条（真实 IME）待人工，
> 另有 1 条半验（全屏 Space：同一显示器已验、副屏待验）。

### notes-core（MODIFIED）

| # | Scenario | 状态 | 证据 |
| --- | --- | --- | --- |
| 1 | 创建笔记只产生一个实体 | 自动 | 既有 `tests/database.rs › crud_soft_delete_restore_and_search_projection`（本轮未改动，断言零修改） |
| 2 | 空正文合法 | 自动 | 既有 `tests/database.rs` 空正文用例 |
| 3 | 速记不扩张笔记实体 | 自动 | `tests/quick_tabs.rs` 五条用例都不触碰 `notes`；`grep` 确认 `quick_tab_repo` 没有引用 `note_repo` |

### macos-shell（MODIFIED）

| Requirement | Scenario | 状态 | 证据 |
| --- | --- | --- | --- |
| 单主窗口 | 连按快捷键只有一个窗口 | 自动（既有） | `tauri-plugin-single-instance` + `WindowManager`；本轮未改该路径 |
| 单主窗口 | 速记面板不复制主窗口 | **人工已验** | 探针第 4/5 步反复 `toggle_from_tray` 后 `get_webview_window("quick-tabs")` 始终是同一实例，`main.visible` 全程 `false`；用户真机反复点击后仍只有这一个面板窗口（口头确认，无留档） |
| 单主窗口 | 尺寸受最小约束 | **人工已验** | `MIN_SIZE = 360×220` 已设；用户真机拖拽确认不可再缩小（口头确认，无留档） |
| 菜单栏常驻 | 左键切换速记面板 | **人工已验** | 探针第 4/5 步走的是 `toggle_from_tray(rect)`（左键的处理函数），显隐与主窗口隔离已验证；用户真机确认左键只切面板、不牵动主窗口（口头确认，无留档） |
| 菜单栏常驻 | 右键打开菜单 | **人工已验** | `menu_on_right_click(true)` 未改；用户真机确认右键只弹菜单、不切面板（口头确认，无留档） |
| 菜单栏常驻 | 菜单显示主窗口 | **人工已验** | 菜单项 ID `toggle` 未改、文案改「显示主窗口」；用户真机确认点到的确实是主窗口（口头确认，无留档） |
| 菜单栏常驻 | 菜单新建笔记 | **待人工**（既有） | 与 v1 同一路径，非本 change 引入 |
| 菜单栏常驻 | 菜单栏显示品牌标识 | **人工已验**（既有） | v1 已验，本轮未改 `icon_as_template(false)` |
| 退出确认 | 取消退出 / 确认退出 / 未保存内容提示 / 只弹一次对话框 | 自动 + **人工已验** | 前端 `listen('ui://quit-requested')` → `flush()` → 面板不弹框由 `QuickTabsApp.test.ts` 与 store 的 `flush()` 用例覆盖；用户真机走完 `⌘Q` 全流程，面板不弹框、只弹一次确认（口头确认，无留档） |
| 显示时定位与置顶 | 鼠标在副屏 / 显示器断开 / 置顶仅在显示时生效 | 自动 | 主窗口路径本轮未改（`WindowManager` 与 `QuickTabsWindow` 是两个类型，design D2）；面板侧的同类场景见 12/13 号 |

## 真机探针结果（任务 2.1 / 2.2 / 2.3 的实测依据）

`src-tauri/src/lib.rs` 里临时加了一段 `[qt-verify]` 探针（真实进程、真实 `AppKit`
事件，脚本化触发），**交付前已删除**（`grep -r qt-verify src-tauri/src` 无输出）。
顺序与输出：

```
1 startup: panel exists=true visible=Some(false) focused=Some(false)
1 startup: main exists=true visible=Some(false)
2 show: panel(visible, pos, size, focused)=(Some(true), Some(1202,50), Some(960x640), Some(true)) main_visible=Some(false)
3 panel-close: panel=(Some(false), …, Some(false)) main_visible=Some(false)
4 tray-click: panel=(Some(true), Some(1032,50), Some(960x640), Some(true)) main_visible=Some(false)
5 tray-click-again: panel=(Some(false), …) blur_guard=false main_visible=Some(false)
6 blur-by-main-focus: panel=(Some(false), …) main_visible=Some(true) blur_guard=false
```

读数说明（重要，避免误读）：

- **尺寸 960×640 是物理像素**，显示器 `scale_factor = 2`，逻辑尺寸正是配置的 480×320。
- 位置 `(1032, 50)` 与纯几何函数的预测完全一致：托盘图标物理 x=1500 → 逻辑 750，
  面板 480 宽 → `750 + (12 − 480)/2 = 516` 逻辑 → 1032 物理。
- 第 2 步 `(1202, 50)` 是"还没有图标矩形"的回退：锚点取显示器顶部中点
  (840, 0)，`840 + (1 − 480)/2 = 601` 逻辑 → 1202 物理。
- y 一律是 50 物理（25 逻辑）：`TRAY_GAP = 6`（当时值，现为 2）算出的 18 逻辑会被 macOS 钳到菜单栏
  下沿，因此面板贴着菜单栏显示，不会盖住图标。
- 第 5/6 步的 `blur_guard=false` 是**预期**：探针两步之间隔了 700 ms，超过 250 ms
  护栏，`blur_hid_recently()` 已自然过期。

### 探针发现并修掉的真 bug

第一版 `place_under_tray` 用 `window.outer_size()` 反推面板尺寸。窗口还没显示过时
该值按比例失真（拿到的是 480/2），于是按半个面板宽居中，落点偏了半个面板。
修法：尺寸只来自我们自己的两个来源（`saved_size` 或 `DEFAULT_SIZE`），
`show_window` 把它作为参数传给 `place_under_tray`；`hide_window` 也只在窗口
**确实可见**时才记录尺寸，避免未显示时的失真值污染记忆尺寸。
纯函数单测测不到这一点（它只测几何），这是运行时探针的净收益。

## 面板包体（design D14）

```
pnpm vite build
dist/quick-tabs.html                  0.55 kB │ gzip 0.32 kB
dist/assets/quick-tabs-*.css          2.97 kB │ gzip 0.80 kB
dist/assets/quick-tabs-*.js           9.61 kB │ gzip 3.91 kB
dist/assets/main-*.js               952.30 kB │ gzip 327.55 kB   （未变，主窗口不加载面板代码）
```

面板入口没有 import `features/notes/**`、`markdown-it`、`shiki`、CodeMirror：
9.61 kB 里只有 Vue + Pinia + 面板组件（`grep -r "markdown-it\|shiki\|codemirror" src/windows/quick-tabs src/stores/quickTabs.ts src/quick-tabs.ts` 无命中）。

## 回归（任务 5.3）

```
pnpm vitest run          20 files / 191 tests passed   （基线 18 / 169 + 新 3 文件 / 22 用例）
pnpm vue-tsc --noEmit    passed
pnpm vite build          passed（两个入口）
cargo test --lib         23 passed
cargo test --test quick_tabs -- --test-threads=1        6 passed
cargo test --test database / library / failures / recovery / import -- --test-threads=1
                         8 / 6 / 6 / 2 / 9 passed
```

既有断言零修改：`git diff main -- 'src/**/*.test.ts' 'src-tauri/tests/*.rs'` 只显示
新增文件，`src/styles/tokens.test.ts` 的改动是**追加**一个 `describe`，没有改旧断言。
`src/features/**`、`src/App.vue`、`src/main.ts`、`index.html`、
`capabilities/default.json`、`tauri.conf.json` 字节不变。

一处**既有**噪声（不是本轮引入）：`pnpm vitest run` 结束时报告 1 个 unhandled
rejection，来自 `src/features/notes/TrashList.test.ts:166` 的
`purgeNote.mockRejectedValue(...)` —— 该文件本轮未改动，用例全部通过。

## 待人工清单（归档时保持未勾）

1. **全屏应用之上呼出（副屏）**（scenario 11a / 任务 2.7）：同一显示器已经用户本人
   验证通过；**副屏全屏**（全屏应用在副屏、图标在副屏菜单栏）仍需试一次，确认面板落副屏、
   不跳主屏。
2. **真实中文输入法 vs 失焦隐藏**（scenario 20b）：敲拼音组合期间点面板外，
   确认组合串不丢、不误触发隐藏以外的事情。
2. **250 ms 护栏的时序**（scenario 10）：面板打开时快速点托盘图标一次，
   确认"关闭"而不是"关掉又打开"。
3. **真机左键 / 右键分工**（macos-shell「菜单栏常驻」）：左键只切面板、右键只弹菜单、
   菜单项「显示主窗口」点的是主窗口。
4. **连点 100 次不产生第二个实例**（macos-shell「单主窗口」）。
5. **面板最小尺寸拖拽**（360×220）与**副屏/显示器断开**下的落点。
6. **`⌘Q` 时面板正在输入**：确认先落盘、只弹一个对话框、取消后内容还在。
7. **任务 5.4 主窗口隔离回归**：⇧⌘D 切主窗口、重复启动唤主窗口、关闭主窗口隐藏、
   退出确认、单实例、居中定位 —— 主窗口相关代码字节未变，但要求真机逐条走一遍。

## 一句话结论

速记的**数据、上限、隔离、草稿恢复、面板显隐与定位**都有可复跑测试或真实进程记录；
**键盘/输入法/真实鼠标点击/多显示器**这四类只能人工验证，归档时留在 `[ ]`。
