## 1. 配置层：把尺寸写进 `AppConfig`

- [x] 1.1 `services/library_service.rs`：新增 `WindowSize { width: f64, height: f64 }`（camelCase、`Serialize`/`Deserialize`）与 `AppConfig` 的 `main_window_size: Option<WindowSize>` / `quick_tabs_size: Option<WindowSize>`（两者 `#[serde(default)]`，`AppConfig` 保持 `rename_all = "camelCase"` 与整体 `default`）。验证：`cargo test --lib` 通过；新增单测断言 `{"libraryPath":"/x"}` 这种旧配置反序列化成功且两个尺寸为 `None`，以及含两个尺寸的 JSON 往返后相等。
- [x] 1.2 写入串行化（design D5 / Risks）：所有 `AppConfig` 的读改写走同一份内存副本 + 单次 `save_config`，不出现两处各自 `load → 改 → save`。验证：`grep -n "save_config" src-tauri/src` 只有一处业务调用点（`library_service` 内部），命令层通过共享状态更新字段；`cargo build` 零 warning。

## 2. 尺寸收敛（纯函数 + 单测）

- [x] 2.1 `platform/macos/monitor.rs` 新增 `clamp_window_size(saved: Option<(f64, f64)>, default: (f64, f64), min: (f64, f64), area: (f64, f64, f64, f64)) -> (f64, f64)`：`None` → 调用方给的默认值经同一套收敛；逐轴 `max(min)` 再 `min(area)`。验证：`cargo test --lib` 新增 4 条单测（比最小尺寸小、比屏幕大、正常值原样、`None` 走默认），全部通过。

## 3. 主窗口：跨重启恢复尺寸

- [x] 3.1 `platform/macos/window_manager.rs`：`saved_size` 的初值来自 `AppConfig.main_window_size`（启动时注入），`hide_window` 记尺寸后同步写回配置。验证：`cargo test --lib` 通过；真机「拖到 1320×860 → 隐藏 → 退出 → 重启 → 唤出」仍是 1320×860（真机部分见 7.1）。
- [x] 3.2 显示路径用 `monitor::clamp_window_size` 收敛后再 `set_size`（design D4）：最小 760×480、最大不超当前显示器可见区域。验证：把 `config.json` 的 `mainWindowSize` 手工改成 `{"width": 3000, "height": 3000}` 后启动，窗口四边都在屏幕内；改成 `{"width": 100, "height": 100}` 后启动，窗口不小于 760×480（几何部分由 `clamp_window_size` 单测覆盖；真机部分见 7.3）。
- [x] 3.3 配置文件缺失/损坏/不可写不阻断：`load_config` 失败按现状回退默认；写盘失败只 `eprintln!`，隐藏与退出照常。验证：`ConfigCache::load` 的错误分支已回退默认并只打日志（真机「非法 JSON 启动」待人工）；写盘失败只 `eprintln!` 的路径已在 `hide_window` 里（只读文件场景见 7.4）。

## 4. 速记面板：跨重启恢复尺寸

- [x] 4.1 `platform/macos/quick_tabs_window.rs`：`saved_size` 初值来自 `AppConfig.quick_tabs_size`，`hide_window` 记尺寸后写回配置；`show_window` 用 `monitor::clamp_window_size` 收敛（最小 360×220，默认 480×320）。验证：`cargo test --lib` 通过；真机「拖到 640×420 → 隐藏 → 退出 → 重启 → 点菜单栏图标」仍是 640×420（真机部分见 7.2）。
- [x] 4.2 坐标仍然不持久化（design Non-Goals）：`config.json` 里没有坐标字段，`place_under_tray` 依旧每次现算。验证：`grep -n "position" openspec/changes/fix-window-size-and-caret/specs/quick-tabs/spec.md` 明确 MUST NOT；`grep -n "x\|y" src-tauri/src/services/library_service.rs` 的 `AppConfig` 无坐标字段。

## 5. 前端：打开即停在第一行

- [x] 5.1 `src/windows/quick-tabs/QuickTabsApp.vue` 的 `focusEditor()`：`nextTick` → `focus()` → `setSelectionRange(0, 0)` → `scrollTop = 0`（design D6）；所有重新获焦路径（打开、切 Tab、新建、重命名收起）复用同一函数。验证：`QuickTabsApp.test.ts` 新增用例——带 200 行内容挂载后触发切 Tab，断言 `editor.selectionStart === 0 && selectionEnd === 0`；既有 10 条用例断言零修改、全部通过（已做反向验证：去掉修复后该用例失败）。
- [x] 5.2 `pnpm vue-tsc --noEmit` 与 `pnpm vitest run src/windows/quick-tabs` 通过。

## 6. 收口

- [x] 6.1 全量回归：`cargo test --lib`、6 个集成测试文件（`--test-threads=1`）、`pnpm vitest run`、`pnpm vue-tsc --noEmit`、`pnpm vite build`、`openspec validate --all --strict`。验证：全部通过，既有断言零修改。注意 `pnpm vitest run` 退出码 1 来自 `TrashList.test.ts:166` 既有 unhandled rejection，用例本身 192/192 全过（见 verification.md）。
- [x] 6.2 `verification.md`：两条需求 → 证据矩阵（自动 / 人工待验），记明「重启后尺寸」与「打开停在第一行」的像素/时长证据形式。验证：`openspec validate fix-window-size-and-caret --strict` 通过。
- [x] 6.3 文档：`CHANGELOG.md` 增两条修复说明。验证：条目存在且与 spec 表述一致。

## 7. 真机确认（未做，勾选即通过）

- [x] 7.1 主窗口拖到 1320×860 → 隐藏 → 退出 → 重启 → 唤出，仍是 1320×860（用户本人真机确认「窗口大小可以保存了」，无截图/录屏留档）
- [ ] 7.2 速记面板拖到 640×420 → 隐藏 → 退出 → 重启 → 点菜单栏图标，仍是 640×420
- [ ] 7.3 把 `config.json` 的 `mainWindowSize` 手改成 `{"width": 3000, "height": 3000}` 后启动，窗口四边仍在屏内；改成 `{"width": 100, "height": 100}` 后启动，窗口不小于 760×480
- [ ] 7.4 把 `config.json` 写成非法 JSON 后启动，应用以默认尺寸正常显示；把它设为只读后隐藏窗口，窗口正常隐藏、进程存活
- [ ] 7.5 速记正文写 200 行后关闭再打开，看到的是第一行且光标在行首；「视图滚到顶部」在 jsdom 里无布局，只能人看
- [ ] 7.6 主窗口与面板都调整过尺寸后切一次库，退出重启，两个尺寸都还在（`ConfigCache` 不互相覆盖字段）
