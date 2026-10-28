# Design — hide-main-window-titlebar

## Context

主窗口由 `tauri.conf.json` 建档（`label = "main"`、`decorations` 默认 true、`title: "Notes"`），布局在 `src/windows/MainWindow.vue`：`.shell` 是 `display: flex` 的一行，依次是 `Sidebar`（宽度由 `settings.sidebarWidth` 驱动、右分隔线）、`SidebarSplitter`、`ContentPane`，后面跟着两个 `position: fixed; inset: 0` 的对话框。

上一轮（`93baf0d`）已把主窗口换成全屏辅助面板（`NSPanel` 子类 + `NonactivatingPanel` + `CanJoinAllSpaces | FullScreenAuxiliary`），标题栏、红绿灯是**样式位**保住的（`titled | closable | miniaturizable | resizable`），换类不碰它们。

tao 的 `titleBarStyle: "Overlay"` 等价于 `setTitlebarAppearsTransparent(true)` + `set_fullsize_content_view(true)`（`tauri-runtime-wry` 源码），`hiddenTitle: true` 等价于 `setTitleVisibility(Hidden)`。三者都只改外观属性：样式位里的 `titled` 仍在，所以红绿灯、圆角、拖拽、双击缩放都还是系统行为。

## Goals / Non-Goals

**Goals**

- 主窗口不再有系统标题栏（没有标题文字、没有标题栏底色），内容顶到窗口最上沿。
- 保留红绿灯与全部窗口级语义（圆角、阴影、⌘W、⌘Q、双击缩放、最小化/缩放按钮）。
- 顶部有一条**明确**的拖拽区，用户仍能拖窗口。

**Non-Goals**

- 不做无边框窗口（不改 `decorations`、不引入自绘标题栏、不改圆角来源）。
- 不改窗口类、不改层级、不改 `prepare_main_window` 的面板配方。
- 不动侧栏与正文的内部布局（侧栏 `.head`、`NoteHeader` 的 padding 都不改）。
- 不动速记面板（它本来就是 `decorations: false`）。

## Decisions

### D1 用 `titleBarStyle: "Overlay"` + `hiddenTitle: true`，不改 `decorations`

Overlay 保留样式位里的 `titled`，所以窗口仍是"有标题栏的窗口，只是标题栏透明且没有标题"：红绿灯由系统画在内容之上，圆角与阴影来自系统窗口本身，双击标题栏缩放、⌘W、窗口最小尺寸全都不受影响。

被否决的方案：`decorations: false`（无边框）。它会去掉红绿灯与系统圆角/阴影，需要自绘拖拽区与 CSS 圆角，还会改掉刚验收的面板样式位语义（`NonactivatingPanel` 之外再动样式位），风险与收益不成比例。

### D2 顶部补一条 32px 拖拽条，左段跟随侧栏

`titleBarStyle: "Overlay"` 之后内容从 y=0 开始，红绿灯会压在侧栏第 11–26px 高度上，而侧栏 `.head` 顶部只有 12px padding、品牌名从 x=14 开始——直接重叠。

做法：`.shell` 从「一行」改成「一列」：先一条 `.titlebar`（`flex: 0 0 auto; height: 32px`），再是原来的三栏 `.body`（`flex: 1`）。

- `.titlebar` 左段宽度绑定 `settings.sidebarWidth`、底色 `--bg-sidebar`、右分隔线 `1px solid var(--border)`，与侧栏完全对齐；右段 `flex: 1`、底色 `--bg`。
- 两段都带 `data-tauri-drag-region`。
- 32px 是"红绿灯（约 12pt 高，垂直中心在 y≈19）+ 一点余量"的量；与改造前 28px 系统标题栏相比总高度基本持平，正文可用高度不缩水。

不改侧栏与正文内部 padding：拖拽条本身提供了那 32px，标题栏消失省下的空间正好被它用掉，视觉上仍是「一条顶栏 + 内容」。

### D3 拖拽靠 `data-tauri-drag-region`，因此必须补一条 capability

Tauri v2 的拖拽区由注入脚本处理：`mousedown` 命中带 `data-tauri-drag-region` 的元素时 `invoke('plugin:window|start_dragging')`。该命令不在 `core:window:default` 里（`acl-manifests.json` 的默认集只有只读查询类命令），所以 `capabilities/default.json` 要显式加 `core:window:allow-start-dragging`，否则控制台会报权限错误、拖不动。

没有选 `NSWindow.setMovableByWindowBackground(true)`：它让整个窗口背景可拖，会和正文里的文本选择、编辑器拖选抢事件。

### D5 必须开 `acceptFirstMouse`，否则窗口未激活时第一次拖不动

开发机实测（合成 `leftMouseDown/Dragged/Up` 事件 + `CGWindowListCopyWindowInfo` 读窗口原点）：

| 状态 | `acceptFirstMouse` | 结果 |
| --- | --- | --- |
| 主窗口已激活 | 默认 `false` | 拖动成功（原点精确位移 +60 / +40） |
| 另一应用在前台（访达激活） | 默认 `false` | **不动**，且前台应用不变 |
| 另一应用在前台 | `true` | 拖动成功（+60 / +40），并顺手把本应用激活 |

原因：wry 的 `WryWebView` 用 `acceptsFirstMouse:` 返回该配置（默认 `false`），窗口未激活时第一次 mouse-down 只用于激活应用、不投递给 webview，于是 `data-tauri-drag-region` 的注入脚本根本没收到事件。这恰好是本需求的主场景——主窗口通常是「别的东西在前台时用快捷键唤起」，所以默认值不可接受。

选 `acceptFirstMouse: true` 而不是在 show 路径里补 `set_focus()`：`set_focus()` 在速记那一轮已被删除（它会把 App 抢到前台，配合全屏面板语义会打架），而 `acceptFirstMouse` 只影响「点击是否穿透到 webview」，不动激活时序。副作用是未激活窗口上的第一下点击会直接生效（点列表就选中、点正文就落光标），这与 macOS 常见应用一致。

### D4 对话框仍覆盖整窗

`SettingsDialog` / `ImportDialog` 是 `position: fixed; inset: 0`，相对视口定位，与 DOM 层级无关，所以它们照旧盖住包括拖拽条在内的整个窗口，不需要跟着挪进 `.body`。

## Risks / Trade-offs

- **红绿灯与拖拽条的对齐只能靠看**：32px 是按标准红绿灯尺寸估的，dev 机上截图核对后如偏差则调 `.titlebar` 高度（纯 CSS 一处）。
- **标题栏文字消失后，窗口在「窗口」菜单 / Mission Control 里仍显示 `title: "Notes"`**：`hiddenTitle` 只影响窗口自身绘制，不影响 `NSWindow.title`，这是想要的（辅助功能仍能读出名字）。
- **权限多一条**：`allow-start-dragging` 只允许"让窗口开始拖动"，无其他副作用；漏加的表现是拖不动 + 控制台报错。
- **顶部 32px 在正文侧是纯色空条**：与改造前系统标题栏占据的位置相同，不额外损失空间。

## Migration Plan

无数据迁移。纯前端 + 配置改动，回滚 = revert 提交（三处）。

## Open Questions

无。对齐像素在实现后用截图核对，属实现细节，不影响 spec 语义。
