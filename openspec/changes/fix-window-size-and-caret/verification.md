# Verification — fix-window-size-and-caret

证据约定与 D20 一致：**只有实际跑过 / 实际点过才写通过**。本机没有辅助访问与屏幕录制权限，
所以「真机重启后尺寸」这类条目只能由用户本人操作后口头确认，取证形式写明为「无截图/录屏留档」。

## 1. 自动证据（本人实跑）

| 项 | 命令 | 结果 |
| --- | --- | --- |
| Rust 单元测试 | `cargo test --lib` | **29 passed**（原 23 + 新增 6：4 条 `clamp_window_size`、3 条配置往返/缓存；删除 1 条随 `clamp_to_area` 一起下线的用例） |
| 编译告警 | `cargo build` | 0 warning / 0 error |
| 集成测试 `quick_tabs` | `cargo test --test quick_tabs -- --test-threads=1` | 6 passed |
| 集成测试 `database` | 同上 | 8 passed |
| 集成测试 `library` | 同上 | 6 passed |
| 集成测试 `failures` | 同上 | 6 passed |
| 集成测试 `recovery` | 同上 | 2 passed |
| 集成测试 `import` | 同上 | 9 passed |
| 前端测试 | `pnpm vitest run` | 20 files / **192 passed**（原 191 + 1 新增） |
| 类型检查 | `pnpm vue-tsc --noEmit` | exit 0 |
| 生产构建 | `pnpm vite build` | exit 0；`quick-tabs` 仍 9.66 kB JS / 3.79 kB CSS（gzip 3.95 / 0.97） |
| OpenSpec | `openspec validate --all --strict` | 通过 |

**`pnpm vitest run` 的退出码是 1，但用例 192/192 全过**：失败来自 `src/features/notes/TrashList.test.ts:166`
既有的 unhandled rejection（`database is locked`，测试故意 mock 的失败路径没有被 await 掉）。
这是本轮之前就存在的噪声，与本改动无关，按项目约定不顺手修。

### 新增测试确实有效（做过反向验证）

`QuickTabsApp.test.ts`「puts the caret at the very beginning…」在**去掉** `setSelectionRange(0, 0)`
两行后确实失败（`expected 2004 to be +0`），加回后通过——不是为了凑数写的空断言。

## 2. 需求 → 证据矩阵

### `macos-shell / 显示时定位与置顶`（尺寸持久化 + 收敛）

| 场景 | 证据 |
| --- | --- |
| 调整尺寸后重启仍是上次的尺寸 | **待人工**：需真机「拖到 1320×860 → 隐藏 → 退出 → 重启 → 唤出」。代码路径已就位（`WindowManager::new` 用 `ConfigCache::size(Main)` 播种，`hide_window` 回写），但**未在真机上跑过完整重启循环** |
| 保存的尺寸比当前屏幕更大 | 自动：`monitor::clamp_window_size` 单测 `remembered_size_is_pulled_onto_a_smaller_display`（2000×1400 存值 + 1440×900 屏 → 1440×900） |
| 尺寸配置损坏不阻断启动 | 部分自动：`ConfigCache::load` 的错误分支回退默认并只打日志；`old_config_without_sizes_still_deserializes` 覆盖"没有尺寸字段的旧配置"。**「把 config.json 写成非法 JSON 后启动仍是默认尺寸」未在真机跑过** |
| 鼠标在副屏 / 显示器断开 / 置顶仅在显示时生效 | 既有场景，本轮未改这些路径；`centered_origin` 单测仍覆盖副屏与负坐标 |

### `quick-tabs / 速记面板的定位`（尺寸持久化 + 收敛）

| 场景 | 证据 |
| --- | --- |
| 调整面板尺寸后重启仍是上次的尺寸 | **待人工**：同上，需真机重启循环 |
| 保存的面板尺寸小于最小尺寸 | 自动：`remembered_size_is_lifted_to_the_minimum`（200×120 → 360×220） |
| 副屏图标在副屏弹出 / 显示器断开 | 既有场景，几何与 `tray_anchor_origin` 未变，单测仍绿 |
| 坐标仍不持久化 | 自动：`AppConfig` 只有 `libraryPath` / `mainWindowSize` / `quickTabsSize` 三个字段，无坐标；`config_round_trips_both_sizes` 断言序列化结果只含尺寸 |

### `quick-tabs / 速记面板打开时的插入点`（新增）

| 场景 | 证据 |
| --- | --- |
| 打开较长的速记 → 停在第一行 | 自动（结构）：单测断言切 Tab 后 `selectionStart === 0 && selectionEnd === 0`；**「视图滚到顶部」在 jsdom 里没有布局，`scrollTop` 恒为 0，无法自动取证** → 视觉部分**待人工** |
| 切换到另一条速记 → 第一条、插入点最前 | 自动：同上用例 |

## 3. 待人工确认清单（不勾选即未通过）

1. 主窗口拖到 1320×860 → 隐藏 → 退出 → 重启 → 唤出，仍是 1320×860。
2. 速记面板拖到 640×420 → 隐藏 → 退出 → 重启 → 点菜单栏图标，仍是 640×420。
3. 把 `config.json` 的 `mainWindowSize` 手改成 `{"width": 3000, "height": 3000}` 后启动，窗口四边仍在屏内。
4. 速记正文写 200 行后关闭再打开，看到的是**第一行**且光标在行首（不是最后一行）。
5. 主窗口与速记面板同时调整过尺寸后切一次库（新建或打开另一个库），退出重启，两个尺寸都还在
   （验证 `ConfigCache` 不互相覆盖字段）。

## 4. 已知的取舍

- 光标在正文开头意味着接着输入会**插在最前面**，不是文末（design D7，用户明确选择）。
- 进程被强杀（崩溃 / 活动监视器结束进程）时最后一次尺寸不落盘——写盘时机是"隐藏时"，不是"每次 resize"（design D3）。
