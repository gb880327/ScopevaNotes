## Why

两个真机使用中暴露的小问题，都影响「下次打开还在原样」这件事：

1. **窗口尺寸不落盘**。主窗口与速记面板的尺寸只存在进程内存（`WindowManager::saved_size` / `QuickTabsWindow::saved_size`），所以进程内拖大还记得，**重启或升级版本就回默认**（1100×720 与 480×320）。用户每次都要重新拖，反复发生。
2. **速记面板打开时光标在末尾**。`focusEditor()` 只调 `textarea.focus()`，浏览器把插入点放到文本末尾并滚到底部，所以每次打开看到的都是最后一行，而不是要接着写的第一行。

两条都不是新功能，是既有契约「恢复上次尺寸」与「显示后让编辑区获得焦点」没有真正兑现。

## What Changes

- 窗口尺寸持久化到应用配置 `config.json`（与库指针同一个文件，`AppConfig` 增加两个可选字段）：
  - 主窗口：隐藏时记尺寸，下次显示时恢复；仍 MUST NOT 保存坐标（沿用既有规则）。
  - 速记面板：隐藏时记尺寸，下次显示时恢复；仍 MUST NOT 保存坐标。
  - 读取时做合法性收敛：小于最小尺寸或超出当前显示器可见区域的值 MUST 被收敛到可用范围，而不是让窗口跑到屏幕外。
  - 配置文件缺失/损坏/不可写 MUST NOT 阻断启动或隐藏窗口，只回退默认值。
- 速记面板打开时把插入点放到正文开头并把视图滚到顶部，使每次打开都能看到第一行。
- 不改变任何既有交互：托盘左右键分工、失焦隐藏、250ms 护栏、全屏 Space 行为、草稿协议、标签栏布局均不动。

## Capabilities

### New Capabilities

无。

### Modified Capabilities

- `macos-shell`：`显示时定位与置顶` —— 「恢复上次窗口尺寸」从仅进程内有效改为跨重启持久化，并明确非法尺寸的收敛与写盘失败不阻断。
- `quick-tabs`：`速记面板的定位` —— 面板尺寸从「本次运行内记忆」改为跨重启持久化，同样明确非法尺寸收敛；新增 requirement `速记面板打开时的插入点` —— 打开即停在正文第一行。

## Impact

- Rust：`src-tauri/src/services/library_service.rs`（`AppConfig` 加字段 + 旧配置兼容）、`src-tauri/src/platform/macos/window_manager.rs`、`src-tauri/src/platform/macos/quick_tabs_window.rs`、`src-tauri/src/state.rs`/`lib.rs` 的串接。
- Rust 测试：尺寸收敛的纯函数单测（新增，放在 `monitor.rs` 或同等纯计算模块），以及在 `src-tauri/tests/` 新增配置往返用例。
- 前端：`src/windows/quick-tabs/QuickTabsApp.vue` 的 `focusEditor()`。
- 配置：`config.json` 新增可选字段（旧文件无这两个字段 MUST 仍能读；新文件被旧版本读取时旧版本忽略未知字段）。
- 不改数据库 schema，不需要迁移；不改 IPC 命令集合（除非串接需要，届时只说新增了哪个）。
