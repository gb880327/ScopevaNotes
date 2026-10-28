## 1. 主窗口加入全屏 Space

- [x] 1.1 `platform/macos/window_manager.rs` 新增 `join_fullscreen_spaces(&WebviewWindow)`：读当前 `collectionBehavior`，OR 上 `CanJoinAllSpaces | FullScreenAuxiliary` 后写回；`ns_window()` 失败返回 `Err`，由调用方降级
- [x] 1.2 `lib.rs` 的 `setup` 里对 `main` 窗口调用一次（主线程、窗口已创建），失败只 `eprintln!` 一行，MUST NOT 阻断启动
- [x] 1.3 层级不动：`show_window` 仍只 `set_always_on_top(true)`、`hide_window` 仍只 `set_always_on_top(false)`，本轮不新增任何 `setLevel`

### 1b. A 档真机不成立后的升级（2026-10-28）

- [x] 1b.1 用临时 Swift 辅助程序（`toggleFullScreen`）制造真实全屏 Space，用临时 Rust 探针 + `CGWindowListCopyWindowInfo` 前到后窗口序列表量化各变体（见 `design.md` D1 表）
- [x] 1b.2 判定只有「非激活 `NSPanel`」可行，`join_fullscreen_spaces` 替换为 `prepare_main_window`：`define_class!` 声明 `ScopevaMainWindowPanel`（`canBecomeKeyWindow` / `canBecomeMainWindow` → `true`）、`object_setClass` 换类、样式 OR `NonactivatingPanel`、`hidesOnDeactivate=false`、`becomesKeyOnlyIfNeeded=false`、`collectionBehavior` 同前
- [x] 1b.3 子类重写 `becomeKeyWindow`：`super` 之后 `NSApp.activateIgnoringOtherApps(true)`，补回"点窗口即激活应用"（菜单栏归属本应用）
- [x] 1b.4 `Cargo.toml` 的 `objc2-app-kit` features 增 `NSApplication`（`activateIgnoringOtherApps` 所在文件受该 feature 门控）
- [x] 1b.5 `lib.rs` 调用点改为 `prepare_main_window`，注释同步；失败仍只 `eprintln!`
- [x] 1b.6 删净临时探针：`src-tauri/src/probe_shell.rs` 删除、`lib.rs` 两处回滚，`grep -rn "probe_shell|SCOPEVA_PROBE" src-tauri/src` = 0
- [x] 1b.7 回归：`cargo build` 0 warning、`cargo test --lib` 29、集成测试 37（`--test-threads=1`）
- [x] 1b.8 用产品代码路径复测全屏 Space：主窗口出现在全屏窗口之上（窗口序号 0 vs 2）且全屏应用状态不变（`isOnActiveSpace=true`、`topInset=0.0`）

## 2. 托盘菜单去掉重复的库入口

- [x] 2.1 `platform/macos/tray.rs` 删除 `library` 的 `MenuItem` 与菜单数组里的位置
- [x] 2.2 `platform/macos/tray.rs` 删除 `on_menu_event` 的 `"library"` 分支
- [x] 2.3 `src/windows/MainWindow.vue` 删除 `listen('ui://open-library', …)`（删完即无发送方）
- [x] 2.4 `grep -rn "open-library" src src-tauri` 必须 0 命中

## 3. 收口

- [x] 3.1 回归：`cargo test --lib`、集成测试（`--test-threads=1`）、`pnpm vitest run`、`vue-tsc`、`vite build`、`openspec validate --all --strict`
- [x] 3.2 写 `verification.md`：需求 → 证据矩阵（哪些靠代码断言、哪些只能真机看）
- [x] 3.3 `CHANGELOG.md` 在 0.6.0（未发布）下补条目

## 4. 真机确认（人做，未做即未通过）

- [x] 4.1 把某个应用切到全屏，再点菜单栏「显示主窗口」：主窗口出现**在该全屏应用之上**，且能**直接打字** —— 用户本人真机确认（2026-10-28，回复「1\2\3测试都ok」），无截图/录屏留档
- [x] 4.2 右键菜单里没有「笔记库」，`设置` 仍能打开且库分区可用 —— 用户本人真机确认（2026-10-28），无截图/录屏留档
- [x] 4.3 主窗口显示期间切换桌面，主窗口仍可见；隐藏后其他应用正常获得焦点 —— 用户接受（未实机验证）：用户于 2026-10-28 确认整体收尾（「可以了」）后关闭本 change，此项未单独逐条验证
- [x] 4.4 换类后普通窗口语义未退化：黄/绿按钮仍可用、拖拽与缩放正常、其他应用在前台时点主窗口菜单栏变成本应用 —— 红/黄/绿按钮由用户真机确认（2026-10-28，在 `hide-main-window-titlebar` 的 4.3 中确认红绿灯行为不变）；「点一下就变成本应用」由开发机合成点击实测（点击后 `lsappinfo front` 由访达变为 macos-notes）；拖拽在补 `acceptFirstMouse` 后由用户复验通过
