# Design — fix-window-size-and-caret

## Context

见 `proposal.md` 的 Why。当前实现的相关约束：

- `WindowManager::saved_size` 与 `QuickTabsWindow::saved_size` 都是 `Mutex<Option<(f64, f64)>>`，进程内有效，退出即失。
- 已有可复用的配置文件通道：`services/library_service.rs` 的 `AppConfig` + `load_config` / `save_config`（先写 `.tmp` 再 `rename`，原子替换），目前只有一个字段 `libraryPath`，头部注释写明"只放库指针与 UI 偏好"——窗口尺寸正属于 UI 偏好，落在同一个文件是设计意图而非破例。
- 两处 `show` 路径都已经把尺寸当作**入参**传入几何函数（`quick_tabs_window.rs` 的注释解释了为什么不能读 `outer_size()`：未 show 过的窗口按 scale 失真），所以"尺寸从哪来"可以整体替换而不动几何。
- `monitor.rs` 已有纯几何单测的先例（`centered_origin` / `clamp_to_area` / `tray_anchor_origin`），可以在无 Mac 环境下测尺寸收敛。

## Goals / Non-Goals

**Goals**

- 尺寸跨重启存活，且"存的值不可用"永远不让窗口跑到屏幕外。
- 写盘失败不影响任何用户可见动作（隐藏窗口、退出）。

**Non-Goals**

- 不持久化坐标（两个窗口都保持既有规则：每次显示重新定位）。
- 不持久化显示器标识、不做多显示器几何记忆。
- 不引入窗口状态插件（`tauri-plugin-window-state`）：它按窗口 label 整体存取，会把坐标一起存下来，与"不记坐标"的契约冲突，且会另起一个状态文件。
- 不改数据库、不加迁移。

## Decisions

### D1 尺寸存进现有 `AppConfig`（`config.json`），不新建状态文件、不用 Tauri Store

`AppConfig` 已经是应用级 UI 偏好的落点，`save_config` 已经是原子写。新开一个 `window-state.json` 只会多一套读写与失败分支。localStorage 也不行：尺寸是原生的、必须在建窗之前就知道，前端拿不到这个时机。

备选：`tauri-plugin-window-state`（否决理由见 Non-Goals）。

### D2 字段形状 = 两个可选对象，缺省即"没存过"

```jsonc
{
  "libraryPath": "…",
  "mainWindowSize":    { "width": 1320.0, "height": 860.0 },
  "quickTabsSize":     { "width": 640.0,  "height": 420.0 }
}
```

- `Option<WindowSize>`，`#[serde(default)]`：旧配置文件没有这两个键 MUST 仍能读（`AppConfig` 已经是 `#[serde(default)]`）。
- 反方向兼容：旧版本应用读新文件时忽略未知字段（serde 默认行为，没有 `deny_unknown_fields`），不会因为多了两个键而报错。
- 用命名对象而不是 tuple：`config.json` 是给人看的，`{"width":…,"height":…}` 比 `[1320.0, 860.0]` 少一次猜。
- 逻辑像素（与两个 `saved_size` 现有语义一致），跨显示器缩放比例变化不会把窗口越存越大。

### D3 只在隐藏/退出时写盘，不在 `Resized` 里边拖边写

拖拽过程中每帧写文件没有收益，还有磁盘抖动风险。窗口隐藏前一定处于可见状态（两处 `hide_window` 都先判 `is_visible()` 才读 `inner_size()`），这一刻的尺寸就是用户拖到的尺寸。

- 写盘失败：`eprintln!` 一行，MUST NOT 让 `hide()` 失败、MUST NOT 弹错。理由与既有 D19 一致——它不是用户动作失败，用户也没法处理。
- 已知损失：进程被强杀（活动监视器、崩溃）时最后那次尺寸不落盘。可接受，写入频率已经降到"每次隐藏一次"。

### D4 读取侧用纯函数收敛：`clamp_window_size`

```
clamp_window_size(saved: Option<(f64,f64)>, default: (f64,f64), min: (f64,f64), area: (f64,f64,f64,f64)) -> (f64,f64)
```

规则：`None` → 用 `default`，然后和"有值"走同一条收敛；逐轴 `max(min)` 后再 `min(area)`。`default` 走同一条路径，是为了让"换到比默认尺寸还小的屏"也收得进去（不然默认值就是唯一能跑出屏幕外的分支）。放在 `monitor.rs`，配单测（无 Mac 可跑）：

- 存的值比最小尺寸小 → 抬到最小尺寸；
- 存的值比当前屏幕大（换了小屏）→ 收进可见区域；
- 存的值正常 → 原样返回；
- `None` → 默认值（并同样受 `min(area)` 约束）。

收敛放在 Rust 而不是相信配置文件内容：`config.json` 是用户可编辑文件，坏值 MUST 变成"窗口还能用"，而不是"窗口 1px 宽"。

### D5 串接方向：`state` 持有配置的只读快照，窗口关闭时回写

启动时 `load_config` 一次，把两个尺寸塞进 `WindowManager` / `QuickTabsWindow`（用现有 `saved_size`，只是初值从配置来）。隐藏时把新尺寸写回 `saved_size` **并**更新配置后 `save_config`。这样几何代码零改动（仍从 `saved_size` 取值），持久化只是"给 `saved_size` 加了初值和回写"。

不加新 IPC 命令、不动前端 DTO。

### D6 插入点：`focus()` 之后显式 `setSelectionRange(0, 0)` + `scrollTop = 0`

`textarea.focus()` 在已有内容时把插入点放到末尾（Chromium 行为），所以顺序是 `nextTick` → `focus()` → `setSelectionRange(0,0)` → `scrollTop = 0`。放在 `focusEditor()` 这一个函数里，所有"重新获得焦点"的路径（打开、切 Tab、重命名收起、新建）自动覆盖。

`selectionStart = selectionEnd = 0` 与 `setSelectionRange(0,0)` 等价，后者同时把 `selectionDirection` 归零，选后者。

### D7 光标在开头意味着"继续写是往顶部插"

用户要的是"每次打开都能看到第一行"，接受由此带来的副作用：接着敲字会插在正文最前面，而不是文末。这是用户明确选定的方向，MUST NOT 自作主张改成"滚到顶部但光标留在末尾"。

## Risks / Trade-offs

- **[写入频率被低估]** 用户频繁开关面板 → 每次隐藏一次 `config.json` 写盘 → Mitigation：写路径与库指针共用同一个原子写；面板开关属于人手速度，量级无风险。
- **[并发写配置文件]** 切库写 `libraryPath` 与隐藏窗口写尺寸可能同帧发生，`load → 改字段 → save` 之间有覆盖风险 → Mitigation：所有 `AppConfig` 写入串行化在同一个 `Mutex<AppConfig>` 之上（内存态是唯一真相，落盘是它的投影），而不是各写各的读-改-写。
- **[旧版本回读新文件]** → 已在 D2 处理：新增键是未知字段，旧版本忽略，不会崩。
- **[坏值把窗口做没]** 用户手工把尺寸改成 1×1 → Mitigation：D4 的最小尺寸收敛。
- **[尺寸收敛与"不记坐标"混淆]** 收敛只用尺寸，不碰坐标；坐标永远由几何函数现算 → 无风险。
- **[`scrollTop` 在 jsdom 里没有布局]** 前端测试只能断言 `selectionStart`/`selectionEnd`，`scrollTop` 恒为 0 → Mitigation：像素观感仍归人工验收，测试只锁插入点。

## Migration Plan

1. `AppConfig` 加字段（向后兼容，旧文件无需迁移）。
2. 启动读取 → 建窗时作为初始尺寸 → 隐藏时回写。
3. 前端插入点修复，随同一轮回归一起验。
4. 回滚策略：删掉读取与回写的两处调用即退回"仅进程内记忆"；`config.json` 里多出来的两个键对回滚后的版本是无害的未知字段。

## Open Questions

无。
