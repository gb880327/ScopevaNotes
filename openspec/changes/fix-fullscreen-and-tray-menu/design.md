## Context

主窗口是真·普通窗口：由 `tauri.conf.json` 建档（`label = "main"`、`alwaysOnTop: false`），显示路径 `WindowManager::show_window` 做四件事 —— `place_on_pointer_monitor`（居中 + 尺寸收敛）、`show()`、`set_always_on_top(true)`、`set_focus()`；隐藏路径 `hide_window` 记尺寸后 `set_always_on_top(false)` + `hide()`。

速记面板走过同一条坑并已解决（`quick_tabs_window::claim_panel_window`）：`object_setClass` 换成 `NSPanel` 子类、`setLevel(NSStatusWindowLevel)`、`setCollectionBehavior(CanJoinAllSpaces | FullScreenAuxiliary)`。当时的核心结论记在 `mem:tauri-nspanel-canbecomekeywindow`：**`visible_on_all_workspaces(true)` 只 OR `CanJoinAllSpaces`，进不了别人正在用的全屏 Space**。

主窗口现在的代码里 `collectionBehavior` 一个字都没设，所以它永远留在自己那个桌面。这不是层级问题（`set_always_on_top` 已给 floating level 3，高于全屏应用窗口的 normal 0），而是 Space 归属问题。

菜单那条是纯冗余：`tray.rs:83` 的 `library` 分支发 `ui://open-library`，`MainWindow.vue:36` 把它接到 `ui.openSettings()`，与 `settings` 分支完全等价。

## Goals / Non-Goals

**Goals**

- 其他应用处于全屏（独立 Space）时，唤出主窗口就能看见它、并且能直接打字。
- 与"置顶"策略保持一致：显示时置顶、隐藏时取消，不引入新的层级。
- 菜单里不再出现与「显示主窗口」重复的库入口，也不留无发送方的事件监听。

**Non-Goals**

- 不改主窗口的窗口类（不换 `NSPanel`），不改标题栏、红绿灯、⌘W、最小化行为。
- 不改速记面板的 panel 语义，不复用 `claim_panel_window`。
- 不改主窗口的定位规则（仍按鼠标所在显示器居中）与尺寸持久化。
- 不新增"库"专用界面；库操作继续在「设置 → 库」里。
- 不处理副屏全屏（用户当前无副屏，无法验证）。

## Decisions

### D1 主窗口只补 Space 归属，不换窗口类

给主窗口的 `NSWindow` 加 `collectionBehavior |= CanJoinAllSpaces | FullScreenAuxiliary`，层级保持 `set_always_on_top` 给的 floating。

理由：主窗口是要被当文档窗口用的 —— 有标题栏、红绿灯要能点、⌘W 要关闭即隐藏、⌘Q 要走退出确认。速记面板那套 `NSPanel` + `NonactivatingPanel` + `canBecomeKeyWindow` 重写是为"呼出即输入、失焦即消失"的浮层定制的，搬到主窗口上会连带影响一批已验收的行为。

**升级判据**：若真机验证发现"看得见但拿不到键盘焦点、无法直接打字"，说明 macOS 不允许非 panel 窗口在全屏 Space 里成为 key，届时不改本决策的语义目标（spec 只承诺"之上可见且可输入"），而是改用与速记一致的 class swap + `canBecomeKeyWindow`。这一步不在本次范围内，需要单独一轮。

### D2 设置时机 = 启动时设一次，不进 show 路径

`collectionBehavior` 是 `NSWindow` 的属性，设置后一直有效；Tauri 的 `show()` / `hide()` / `set_always_on_top()` 都不碰它。所以在 `lib.rs` 的 `setup` 里（窗口已由 `tauri.conf.json` 建好、且在主线程上）设一次即可，避免每次 show 走 `run_on_main_thread` 往返。

`ns_window()` 只能在主线程调，`setup` 满足；失败只 `eprintln!` 一行，不阻断启动（与 `quick_tabs_window::create` 的失败处理一致）。

### D3 菜单项直接删，不留兼容分支

「笔记库…」没有独立业务语义（它就是 `openSettings`），保留任何一个"只显示主窗口"的版本都会与第一项「显示主窗口」重复。删除 `library` 菜单项、`on_menu_event` 的 `library` 分支，以及前端的 `ui://open-library` 监听（删完即刻无发送方）。

`ui://open-settings` 保留不动：它仍是「设置…」菜单项的路径。

## Risks / Trade-offs

- **拿不到键盘焦点**（见 D1 升级判据）：本次只做 A 档，真机不通过就上 class swap。风险由真机确认项 4.1 兜住，不会把未验证的能力写进已完成态。
- **主窗口跟着切 Space**：`CanJoinAllSpaces` 意味着主窗口出现在所有桌面，切 Space 也跟着走。这是用户已接受的语义（与"显示期间置顶"一致），但属于可感知变化，写进 verification 的已知取舍。
- **菜单少一项**：老用户肌肉记忆会落空，库设置要多点一层（设置 → 库）。可接受，菜单项原本就是空壳。
- **`ui://open-library` 事件消失**：仓库内唯一发送方与唯一监听方一并删除。若外部有第三方脚本监听它（无此契约），会静默失效。

## Migration Plan

无数据迁移。纯运行时行为调整，回滚只需 revert 提交。

## Open Questions

- 副屏全屏场景未验（用户当前无副屏）：多显示器时主窗口应出现在鼠标所在显示器的全屏 Space 之上，代码上 `CanJoinAllSpaces` 与具体显示器无关，预期成立但未实测。
