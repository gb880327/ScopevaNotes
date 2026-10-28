# Verification — fix-fullscreen-and-tray-menu

## 1. 需求 → 证据矩阵

| 需求（生效 spec） | 实现位置 | 自动证据 | 真机证据 |
| --- | --- | --- | --- |
| 全屏应用时主窗口 MUST 显示在该全屏应用之上，且 MUST NOT 让用户被动退出全屏 | `window_manager::prepare_main_window`：换类 `ScopevaMainWindowPanel`（`NSPanel` 子类）+ 样式位 `NonactivatingPanel` + `collectionBehavior \|= CanJoinAllSpaces \| FullScreenAuxiliary`，`lib.rs` setup 调用一次 | **开发机实测通过**（§3.1：真实全屏 Space 下主窗口在屏上列表第 0 位、全屏应用仍在第 2 位且 `isOnActiveSpace=true`）；`cargo build` 无警告 | **待真机 4.1** |
| 该能力 MUST NOT 让主窗口退化为不接收键盘的浮层；成为 key 时 MUST 让本应用成为活动应用 | `ScopevaMainWindowPanel` 重写 `canBecomeKeyWindow` / `canBecomeMainWindow` → `true`；`becomeKeyWindow` 里 `super` 后 `NSApp.activateIgnoringOtherApps(true)` | 同一次实测：`isKey=true isMain=true appActive=true`，全屏应用侧 `key=false`（焦点确实转给主窗口）；`cargo test --lib` 29 passed；集成测试 37 passed | **待真机 4.1（能否直接打字）/ 4.4（点窗口菜单栏是否变成本应用）** |
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

## 3. 全屏 Space 实测（2026-10-28，开发机自测）

### 3.1 方法

macOS 不允许脚本把自己的窗口放进"别人正在使用的全屏 Space"，所以两侧都要造：

- **全屏 Space**：临时 Swift 辅助程序（`NSWindow` + `toggleFullScreen`，属另一个 app），每 2 秒打印自身状态，并用 `CGWindowListCopyWindowInfo(.optionOnScreenOnly, .excludeDesktopElements)` 输出**前到后**的窗口栈，从中取 `fsIndex`（辅助程序全屏窗口的位次）与 `notesIndex`（`owner=macos-notes` 且高度 > 200 的窗口位次，`-1` = 根本没画出来）。
- **主窗口侧**：临时 Rust 探针模块（`#[cfg(debug_assertions)]` + `SCOPEVA_PROBE` 门控）在主线程逐跳打印 `visible / isKey / isMain / isVisible / canBecomeKey / level / isOnActiveSpace / occlusionState / styleMask / collectionBehavior / frame / NSApp.isActive`，日志只写 `/tmp/`。

判据三条同时成立才算过：① `notesIndex >= 0`（真被画出来且在全屏窗口之前）；② 辅助程序 `isOnActiveSpace=true` 且 `topInset=0.0`（用户没被踢出全屏、全屏也没被挤掉）；③ 主窗口 `isKey=true`（能直接打字）。

### 3.2 变体结果

| 变体 | 结果 |
| --- | --- |
| A 档（提交 `c0d322c` 的实现：普通 `NSWindow` + `collectionBehavior`） | ❌ `notesIndex=-1`，且辅助程序的 `isOnActiveSpace` 由 `true` 翻成 `false`、`topInset` 由 `0.0` 回到 `25.0` —— 主窗口是靠"把别人的全屏挤掉"上位的，用户被动切回原桌面。**这就是真机失败机制。** |
| `NSPanel` 换类 + 激活（无 `NonactivatingPanel`） | ❌ `notesIndex=-1`、`onActiveSpace=false`：全屏 Space 保住了，但窗口根本没画出来 |
| 同上 + `show` 之后再补 `setLevel(25)` | ❌ 同上（`show` 会先把 level 打回 5，补层级无效） |
| 普通 `NSWindow` + `level=25`，不激活 | ❌ `notesIndex=-1`：不激活也一样画不上去 |
| **`NSPanel` + `NonactivatingPanel` + 激活** | ✅ `notesIndex=0`、`fsIndex=2`、辅助程序 `isOnActiveSpace=true topInset=0.0`、主窗口 `isKey=true isMain=true appActive=true` |
| 产品代码路径复测（`prepare_main_window` 落地后，探针不再自行换类） | ✅ 与上一行一致：`notesIndex=0` / `fsIndex=2` / `topInset=0.0` / `isKey=true` |

普通桌面（不涉全屏）复测同一配方：`visible=true isKey=true isMain=true onActiveSpace=true appActive=true occlusionState=8194`，与改造前一致，未退化。

### 3.3 探针与辅助程序的处置

两侧都是临时件，**已全部删除**：`src-tauri/src/probe_shell.rs` 删除、`lib.rs` 的 `mod`/`start` 两处回滚，`grep -rn "probe_shell\|SCOPEVA_PROBE" src-tauri/src` = 0 命中；Swift 辅助程序与驱动脚本只存在于 `/tmp/`，不在仓库内。

## 4. 只能真机看的项（未做即未通过）

1. 把某个应用切全屏 → 菜单栏「显示主窗口」→ 主窗口出现在**该全屏应用之上**，那个应用**不掉出全屏**。
2. 接上条：主窗口里**能直接打字**（开发机已测到 `isKey=true`，但"人的手感"只能人确认）。
3. 右键菜单里没有「笔记库」，「设置…」仍能打开且库分区可用。（用户 2026-10-28 已确认通过）
4. 主窗口显示期间切换桌面，主窗口仍可见；隐藏后其他应用正常获得焦点。
5. 换类后普通窗口语义：黄/绿按钮、拖拽缩放、其他应用在前台时点主窗口菜单栏是否变成本应用（`becomeKeyWindow` 里的激活是否如期生效）。

## 4. 已知取舍

- `CanJoinAllSpaces` 让主窗口**跟着用户切所有桌面**。这与"显示期间置顶"是同一取向，且是"能在别人全屏 Space 之上出现"的前提，属可感知变化。
- 菜单少一项，库设置要多点一层（设置 → 库）。
- 副屏全屏未验（用户当前无副屏）：代码与具体显示器无关，但未实测。
- §3 的全屏实测目标是"主窗口是否出现在全屏窗口之前"，用窗口栈序号判定；**没有**截图/录屏留档（`screencapture` 在该环境下会出黑帧），口径同 D20。
- 换类到 `NSPanel` 后，最小化/缩放等只能人手点的语义未验证（见 §4 第 5 条）。
