## 1. 主窗口加入全屏 Space

- [x] 1.1 `platform/macos/window_manager.rs` 新增 `join_fullscreen_spaces(&WebviewWindow)`：读当前 `collectionBehavior`，OR 上 `CanJoinAllSpaces | FullScreenAuxiliary` 后写回；`ns_window()` 失败返回 `Err`，由调用方降级
- [x] 1.2 `lib.rs` 的 `setup` 里对 `main` 窗口调用一次（主线程、窗口已创建），失败只 `eprintln!` 一行，MUST NOT 阻断启动
- [x] 1.3 层级不动：`show_window` 仍只 `set_always_on_top(true)`、`hide_window` 仍只 `set_always_on_top(false)`，本轮不新增任何 `setLevel`

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

- [ ] 4.1 把某个应用切到全屏，再点菜单栏「显示主窗口」：主窗口出现**在该全屏应用之上**，且能**直接打字**
- [ ] 4.2 右键菜单里没有「笔记库」，`设置` 仍能打开且库分区可用
- [ ] 4.3 主窗口显示期间切换桌面，主窗口仍可见；隐藏后其他应用正常获得焦点
