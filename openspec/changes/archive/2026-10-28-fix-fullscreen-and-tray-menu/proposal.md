## Why

真机使用暴露两条问题，一条是可见性缺陷，一条是菜单里的空壳项：

1. **主窗口进不了别人的全屏 Space**。速记面板当初就踩过这个坑并已修好（`claim_panel_window` 设了 `canJoinAllSpaces | fullScreenAuxiliary`），但主窗口只做了 `set_always_on_top(true)`（= floating level）。层级再高也没用 —— 主窗口根本不属于当前全屏应用所在的 Space，所以唤出后落在原来的桌面上，用户必须先退出全屏才能看到它。
2. **托盘菜单的「笔记库…」是个空壳**。`tray.rs` 发 `ui://open-library`，而 `MainWindow.vue` 把它接到了 `ui.openSettings()` —— 和「设置…」完全同一条路径。用户点「笔记库」得到的是整窗遮罩的设置对话框，既和菜单文案不符，也和第一项「显示主窗口」行为重复。

## What Changes

- 主窗口在显示时 MUST 出现在其他应用的全屏 Space 之上：通过给主窗口的 `NSWindow` 补 `NSWindowCollectionBehavior::CanJoinAllSpaces | FullScreenAuxiliary` 实现（与速记面板同一套 Space 归属手法，但**不换窗口类**）。
  - 主窗口 MUST 仍是普通可激活窗口：原生标题栏、红绿灯、⌘W、⌘Q 语义不变。
  - 层级策略不变：仍然是显示时 `set_always_on_top(true)`、隐藏时取消，本条只补 Space 归属。
- 托盘菜单删除「笔记库…」项。库的查看与切换只经「设置 → 库」进入。前端同步删除已无发送方的 `ui://open-library` 监听。
- 不改变其他既有交互：托盘左右键分工、速记面板的 panel 语义、失焦隐藏、尺寸持久化、草稿协议均不动。

## Capabilities

### New Capabilities

无。

### Modified Capabilities

- `macos-shell`：`显示时定位与置顶` —— 新增"其他应用全屏时主窗口 MUST 显示在该全屏应用之上"，并锁定"主窗口 MUST NOT 因此退化为不接收键盘的浮层"。
- `macos-shell`：`菜单栏常驻` —— 菜单清单去掉「打开笔记库」，并明确菜单 MUST NOT 提供与其他菜单项重复的入口，库相关操作只经设置。

## Impact

- Rust：`src-tauri/src/platform/macos/window_manager.rs`（新增设置 collection behavior 的函数）、`src-tauri/src/lib.rs`（启动时调用一次）、`src-tauri/src/platform/macos/tray.rs`（删菜单项与分支）。
- 前端：`src/windows/MainWindow.vue`（删 `ui://open-library` 监听）。
- 规格：`openspec/specs/macos-shell/spec.md` 两条 requirement。
- 不改数据库 schema、不改 IPC 命令集合、不改事件列表之外的东西（`ui://open-library` 是删除而非新增）。
- 真机判定项：全屏应用上方的可见性与可输入性只能人看，代码侧只保证 Space 归属与层级。
