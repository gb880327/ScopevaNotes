# Verification — fix-fullscreen-and-tray-menu

## 1. 需求 → 证据矩阵

| 需求（生效 spec） | 实现位置 | 自动证据 | 真机证据 |
| --- | --- | --- | --- |
| 全屏应用时主窗口 MUST 显示在该全屏应用之上 | `window_manager::join_fullscreen_spaces`（`collectionBehavior \|= CanJoinAllSpaces \| FullScreenAuxiliary`）+ `lib.rs` setup 调用一次 | `cargo build` 无警告；本次本地启动进程 12 秒无 panic、无 `[window] full-screen space unavailable` 输出 | **待真机 4.1** |
| 该能力 MUST NOT 让主窗口退化为不接收键盘的浮层 | 未改窗口类、未改 `setLevel`；`show_window` / `hide_window` 与提交 `88a3e64` 逐字节一致 | `cargo test --lib` 29 passed；集成测试全绿（见 §2） | **待真机 4.1（能否直接打字）** |
| 菜单 MUST 至少包含「显示主窗口 / 新建笔记 / 导入 Markdown… / 设置 / 开机启动 / 退出」 | `tray.rs` `Menu::with_items`（删掉 `library` 一项） | 该文件 `MenuItem::with_id` 列表即代码事实；`ui://open-library` 全仓 `grep` = 0 命中 | **待真机 4.2** |
| 菜单 MUST NOT 出现与其他菜单项重复的入口 | 删除 `library` 项与 `on_menu_event` 的 `"library"` 分支、删除前端监听 | `grep -rn "open-library" src src-tauri` → 0 命中 | **待真机 4.2** |
| 库的查看与切换 MUST 只经「设置 → 库」 | `SettingsDialog.vue` 内 `LibrarySettings` 未改动 | 未改前端功能代码（仅删一行死监听） | **待真机 4.2** |

## 2. 实际执行过的检查（2026-10-28，本轮）

| 检查 | 结果 |
| --- | --- |
| `cargo build` | 0 error / 0 warning |
| `cargo test --lib` | **29 passed** |
| `cargo test --tests -- --test-threads=1`（database 8 / library 6 / import 9 / failures 6 / quick_tabs 6 / recovery 2） | 全绿，共 37 passed |
| `pnpm vitest run` | 20 files，**191 passed + 1 failed（既有噪声 `CodeBlock.test.ts` Shiki 懒加载在 load average 15+ 时超时）**；单独重跑该文件 7/7 通过 |
| `vitest` 的 unhandled rejection | 仍是既有噪声 `TrashList.test.ts:166`（`database is locked` 未 await），本轮不修（项目约定） |
| `npx vue-tsc --noEmit` | exit 0 |
| `npx vite build` | exit 0（仅既有 `main` chunk >500 kB 警告） |
| `openspec validate --all --strict` | **10 passed**（9 spec + 本 change） |
| 本地启动冒烟 | 启动 dev 二进制 + vite(1420)，12 秒存活、`/tmp/app.log` 空（无 panic、无 `[window] full-screen space unavailable`），随后终止；未观测窗口外观 |

## 3. 只能真机看的项（未做即未通过）

1. 把某个应用切全屏 → 菜单栏「显示主窗口」→ 主窗口出现在**该全屏应用之上**。
2. 接上条：主窗口里**能直接打字**。若只能看见、点一下才聚焦，说明 A 档（`collectionBehavior`）不足，按 design D1 的升级判据改用 class swap（`NSPanel` + `canBecomeKeyWindow`），需单独一轮。
3. 右键菜单里没有「笔记库」，「设置…」仍能打开且库分区可用。
4. 主窗口显示期间切换桌面，主窗口仍可见；隐藏后其他应用正常获得焦点。

## 4. 已知取舍

- `CanJoinAllSpaces` 让主窗口**跟着用户切所有桌面**。这与"显示期间置顶"是同一取向，且是"能在别人全屏 Space 之上出现"的前提，属可感知变化。
- 菜单少一项，库设置要多点一层（设置 → 库）。
- 副屏全屏未验（用户当前无副屏）：代码与具体显示器无关，但未实测。
- 本地只验证了"启动不炸、属性设置调用未报错"，**没有**验证 `collectionBehavior` 生效后的视觉效果 —— 无截图/录屏留档，和 D20 的口径一致。
