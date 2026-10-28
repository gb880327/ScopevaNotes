# Proposal — hide-main-window-titlebar

## Why

主窗口是靠快捷键（⌘⇧D）与菜单栏唤出/隐藏的，不是靠窗口管理器切换的文档窗口。它现在仍然顶着一条系统标题栏：中间写着「Notes」，左边是红绿灯，下面再另起一行显示品牌名。对这类"按需浮现"的窗口，标题栏只是白占一条高度，而且窗口标题与侧栏品牌名重复。

用户要求：主窗口不要标题栏。

## What Changes

- 主窗口改为「标题栏透明 + 标题隐藏」（`titleBarStyle: "Overlay"` + `hiddenTitle: true`），内容顶到窗口最上方。
- 窗口顶部补一条 32px 的**拖拽条**（`data-tauri-drag-region`），左段与侧栏同底色并延续侧栏分隔线，右段与正文同底色，视觉上仍是窗口顶部但不是系统标题栏。
- `capabilities/default.json` 增加 `core:window:allow-start-dragging`（`data-tauri-drag-region` 通过 IPC 调 `start_dragging`，该权限不在 `core:default` 里）。
- 红绿灯、圆角、阴影、双击缩放、⌘W 关闭即隐藏、⌘Q 退出确认**全部保持现状**——这一轮不碰窗口类（仍是上一轮的全屏辅助面板）也不碰窗口级语义。

## Impact

- 受影响 spec：`macos-shell`（主窗口外观）。
- 受影响代码：`src-tauri/tauri.conf.json`、`src-tauri/capabilities/default.json`、`src/windows/MainWindow.vue`。
- 不受影响：数据层、命令层、其他窗口（速记面板）、设置对话框等固定定位浮层（`position: fixed; inset: 0`，仍覆盖整个窗口）。
- 无数据迁移，回滚 = revert 提交。
