# Verification — hide-main-window-titlebar

## 需求矩阵

| 需求（`spec: macos-shell`） | 实现位置 | 证据 | 状态 |
| --- | --- | --- | --- |
| 主窗口无标题栏：无标题文字、无标题栏底色、内容顶到窗口上沿 | `src-tauri/tauri.conf.json`（`main.titleBarStyle = "Overlay"`、`main.hiddenTitle = true`） | dev 机截图逐像素核对（见 §2） | 通过（开发机） |
| 保留红绿灯/圆角/阴影/双击缩放/关闭即隐藏/退出确认 | 未改 `decorations`、未改窗口类；`prepare_main_window` 未动 | 截图见红绿灯仍在左上；⌘W / ⌘Q 路径未改（`window_manager::hide_window`、退出确认代码零改动） | 红绿灯与几何：开发机通过；⌘W / ⌘Q / 双击缩放：待真机（§3） |
| 顶部拖拽区横跨窗口宽度、左段与侧栏同宽同色并延续分隔线、右段与正文同色 | `src/windows/MainWindow.vue`（`.titlebar` / `.titlebar-side` / `.titlebar-main`） | dev 机截图核对颜色与分隔线位置（见 §2） | 通过（开发机） |
| 内容不被红绿灯遮挡 | 同上（32px 拖拽条把内容推到红绿灯下方） | dev 机截图核对（红绿灯底边 y=20，侧栏首个元素 y=44） | 通过（开发机） |
| 拖拽区能拖动窗口 | `data-tauri-drag-region` + `capabilities/default.json` 的 `core:window:allow-start-dragging` | 权限标识在启动期由 Tauri 校验（写错会启动失败，本次启动正常）；**真实拖动动作未经自动化验证** | 待真机（§3） |

## §1 自动化检查

全部在 `feat/hide-titlebar` 分支、提交前跑：

| 检查 | 命令 | 结果 |
| --- | --- | --- |
| 类型 | `npx vue-tsc --noEmit` | 0 error |
| 前端单测 | `npx vitest run` | 20 files / 192 passed（另有既有噪声 `TrashList.test.ts:166` 的 `database is locked` unhandled rejection，与本改动无关） |
| 前端构建 | `npx vite build` | exit 0，`main-Cs6fWasm.js` 952.51 kB |
| Rust 构建（含 `tauri.conf.json` 编译期校验） | `cargo build` | 0 error 0 warning；配置与 capability 均被接受 |
| Rust 单测 | `cargo test --lib` | 29 passed |
| OpenSpec | `openspec validate --all --strict` | 11 passed（9 spec + 2 未归档 change） |

未改任何 Rust 代码与既有断言；`cargo build` 的配置校验覆盖了 `titleBarStyle` / `hiddenTitle` / `allow-start-dragging` 三处新增字段的合法性。

## §2 视觉核对（2026-10-28，开发机）

方法：临时把 `main.visible` 改为 `true`（截图后已还原为 `false`），`pnpm tauri dev` 起应用，`screencapture -x` 抓全屏，用 PIL 按窗口实际矩形（`CGWindowListCopyWindowInfo` 得 X=290 / Y=178 / 1100×721 点，Retina 2×）逐像素扫描。深色主题。

窗口内实测（单位 pt，自窗口上沿/左沿）：

- 红绿灯：三颗圆点位于 x≈8–19 / 28–39 / 48–59、y≈8–20（未获焦的灰态 `#424245` 附近）。→ 完全落在 32px 拖拽条内。
- 侧栏首个可见元素（`+` 按钮）从 y=44 开始（= 32 拖拽条 + 12 侧栏 padding）；x 70–290 区间在 y∈[2,43] 内**无任何非底色像素**。
- 正文侧（x 310–1095）在 y∈[1,80] 内**无非底色像素**——顶部没有残留文字或控件。
- 底色：拖拽条左段与侧栏同为 `rgb(35,35,38)`，右段与正文同为 `rgb(28,28,30)`，与下方对应面板一致。
- 分隔线：拖拽条内（y=5 / 25 / 30 / 32）与正文内（y=350）的侧栏右缘 x 均在 299 pt，**逐像素对齐**。

结论：标题栏消失、内容顶到上沿、红绿灯不压内容、拖拽条与两栏颜色和分隔线对齐。

已知限制：截图只能证明静态外观与颜色/位置关系，**不能**证明拖拽、双击缩放、⌘W、⌘Q 行为；这些留给 §3。

## §3 待真机确认（不勾选即未通过）

- [ ] 4.1 快捷键唤出主窗口：顶部没有标题文字与标题栏底色，界面从窗口上沿开始
- [ ] 4.2 按住顶部空条能移动窗口；双击空条能缩放
- [ ] 4.3 红绿灯仍在且行为不变（红=隐藏、黄=最小化、绿=全屏）

## 人工确认记录

（暂无。§3 三项需用户本人在真机上操作，本轮未做，因此 `tasks.md` 第 4 组保持未勾选。）
