## Context

主窗口是真·普通窗口：由 `tauri.conf.json` 建档（`label = "main"`、`alwaysOnTop: false`），显示路径 `WindowManager::show_window` 做四件事 —— `place_on_pointer_monitor`（居中 + 尺寸收敛）、`show()`、`set_always_on_top(true)`、`set_focus()`；隐藏路径 `hide_window` 记尺寸后 `set_always_on_top(false)` + `hide()`。

速记面板走过同一条坑并已解决（`quick_tabs_window::claim_panel_window`）：`object_setClass` 换成 `NSPanel` 子类、`setLevel(NSStatusWindowLevel)`、`setCollectionBehavior(CanJoinAllSpaces | FullScreenAuxiliary)`。当时的核心结论记在 `mem:tauri-nspanel-canbecomekeywindow`：**`visible_on_all_workspaces(true)` 只 OR `CanJoinAllSpaces`，进不了别人正在用的全屏 Space**。

主窗口原本 `collectionBehavior` 一个字都没设，所以它永远留在自己那个桌面。这不是层级问题（`set_always_on_top` 已给 floating level 3，高于全屏应用窗口的 normal 0），而是 Space 归属问题。

第一版实现（提交 `c0d322c`）只补了 `collectionBehavior`（下称 A 档）。真机反馈「主窗口还是只会显示在主屏」，随后在开发机上复现并量化（见 D1 的实测表）：A 档不成立。

菜单那条是纯冗余：`tray.rs:83` 的 `library` 分支发 `ui://open-library`，`MainWindow.vue:36` 把它接到 `ui.openSettings()`，与 `settings` 分支完全等价。

## Goals / Non-Goals

**Goals**

- 其他应用处于全屏（独立 Space）时，唤出主窗口就能看见它、并且能直接打字。
- 与"置顶"策略保持一致：显示时置顶、隐藏时取消，不引入新的层级。
- 菜单里不再出现与「显示主窗口」重复的库入口，也不留无发送方的事件监听。

**Non-Goals**

- 不改主窗口的标题栏外观、红绿灯位置、⌘W 关闭即隐藏、⌘Q 退出确认语义（窗口类已按 D1 换成 `NSPanel`，这些语义靠样式位保留而不是靠窗口类保留）。
- 不引入自定义标题栏、不改成无边框。
- 不改速记面板的 panel 语义，不复用 `claim_panel_window`。
- 不改主窗口的定位规则（仍按鼠标所在显示器居中）与尺寸持久化。
- 不新增"库"专用界面；库操作继续在「设置 → 库」里。
- 不处理副屏全屏（用户当前无副屏，无法验证）。

## Decisions

### D1 主窗口改为「全屏辅助面板」：类换成 `NSPanel`，样式加 `NonactivatingPanel`

第一版（A 档）只给普通 `NSWindow` 加 `CanJoinAllSpaces | FullScreenAuxiliary`，真机与开发机都不成立。开发机上用一个临时 Swift 辅助程序制造真实的全屏 Space（`toggleFullScreen`），再用临时 Rust 探针驱动主窗口显示、用 `CGWindowListCopyWindowInfo` 的**前到后窗口序列**做客观判据（不是看截图猜）。结果：

| 变体 | 主窗口类 / 样式 | 主窗口是否出现在全屏窗口之上 | 全屏 Space 是否仍是当前 Space |
| --- | --- | --- | --- |
| A 档（`c0d322c`） | `NSWindow` + `collectionBehavior` | 否 | **否**（用户被切回原桌面） |
| 面板 + 激活 | `NSPanel`，无 `NonactivatingPanel` | **否**（窗口不在屏上列表） | 是 |
| 面板 + 激活 + level 25 | 同上，`setLevel` 在 `show` 之后补 | **否** | 是 |
| 普通窗口 + level 25，不激活 | `NSWindow` | **否** | 是 |
| **面板 + `NonactivatingPanel` + 激活** | `NSPanel` + `NSWindowStyleMaskNonactivatingPanel` | **是**（窗口序号在列表第 0 位，全屏窗口在第 2 位） | **是** |

结论与速记面板当初的结论一致（`mem:tauri-nspanel-canbecomekeywindow`）：**只有非激活面板能在别人的全屏 Space 里被画出来**。普通窗口即使层级更高也只是"没有画上去"，而带激活行为的窗口会把用户拽回自己那个桌面。

因此主窗口按面板配方改造（`window_manager::prepare_main_window`）：

- `object_setClass` 换成 `NSPanel` 子类 `ScopevaMainWindowPanel`（`NSPanel` 不增加实例变量，456 vs 464 字节，raw FFI 换类是安全的）；
- 子类重写 `canBecomeKeyWindow` / `canBecomeMainWindow` 返回 `true`（换类会丢掉 tao 自己的重写）；
- 样式位 OR 上 `NSWindowStyleMask::NonactivatingPanel`；`hidesOnDeactivate = false`、`becomesKeyOnlyIfNeeded = false`；
- `collectionBehavior |= CanJoinAllSpaces | FullScreenAuxiliary`。

层级不动：仍由 `set_always_on_top(true)` 给 floating。实测 floating 已足够压在全屏窗口（layer 0）之上，不必提层级，也就不会与"显示时置顶 / 隐藏时取消"的策略打架。

**被否决的选项**：保持 `NSWindow` 只提层级（实测无效）；`NSPanel` 但不加 `NonactivatingPanel`（实测画不出来）。

### D1b 靠 `becomeKeyWindow` 补回激活行为

`NonactivatingPanel` 的语义是"成为 key 不激活应用"，正好是超层显示需要的，但对文档窗口是副作用：应用不在前台时点窗口，菜单栏会留在原来那个应用上，⌘W / ⌘Q 会打到别人身上。

所以在子类里重写 `becomeKeyWindow`：先 `super`，再 `NSApp.activateIgnoringOtherApps(true)`（`activate` 是 macOS 14+，bundle 目标 12.0，所以用被标记 deprecated 的旧 API 并 `#[allow(deprecated)]`）。这样窗口拿到键盘的同时应用也被激活，菜单栏归属跟普通窗口一致；而在全屏 Space 里激活不会再把用户切走——窗口本身已经在那个 Space 里（上表最后一行就是激活状态下的实测）。

### D2 设置时机 = 启动时设一次，不进 show 路径

`collectionBehavior` 是 `NSWindow` 的属性，设置后一直有效；Tauri 的 `show()` / `hide()` / `set_always_on_top()` 都不碰它。所以在 `lib.rs` 的 `setup` 里（窗口已由 `tauri.conf.json` 建好、且在主线程上）设一次即可，避免每次 show 走 `run_on_main_thread` 往返。

`ns_window()` 只能在主线程调，`setup` 满足；失败只 `eprintln!` 一行，不阻断启动（与 `quick_tabs_window::create` 的失败处理一致）。

### D3 菜单项直接删，不留兼容分支

「笔记库…」没有独立业务语义（它就是 `openSettings`），保留任何一个"只显示主窗口"的版本都会与第一项「显示主窗口」重复。删除 `library` 菜单项、`on_menu_event` 的 `library` 分支，以及前端的 `ui://open-library` 监听（删完即刻无发送方）。

`ui://open-settings` 保留不动：它仍是「设置…」菜单项的路径。

## Risks / Trade-offs

- **`NSPanel` 语义差异**：面板在 AppKit 里比普通窗口少了"最小化/缩放"的一部分默认行为，也可能不进 Window 菜单。样式位保留了 `titled|closable|miniaturizable|resizable`，预期黄/绿按钮仍可用，但这属于只能真机点的事，写进待确认项。
- **样式位可能被 Tauri 改写**：`set_always_on_top`、`show`/`hide` 都不碰 `styleMask`（已按类 API 核对），但若将来 Tauri 内部重建窗口或改 `styleMask`，`NonactivatingPanel` 位会丢，需要重新调用 `prepare_main_window`。
- **主窗口跟着切 Space**：`CanJoinAllSpaces` 意味着主窗口出现在所有桌面，切 Space 也跟着走。这是用户已接受的语义（与"显示期间置顶"一致），但属于可感知变化，写进 verification 的已知取舍。
- **菜单少一项**：老用户肌肉记忆会落空，库设置要多点一层（设置 → 库）。可接受，菜单项原本就是空壳。
- **`ui://open-library` 事件消失**：仓库内唯一发送方与唯一监听方一并删除。若外部有第三方脚本监听它（无此契约），会静默失效。

## Migration Plan

无数据迁移。纯运行时行为调整，回滚只需 revert 提交。

## Open Questions

- 副屏全屏场景未验（用户当前无副屏）：多显示器时主窗口应出现在鼠标所在显示器的全屏 Space 之上，代码上 `CanJoinAllSpaces` 与具体显示器无关，预期成立但未实测。
