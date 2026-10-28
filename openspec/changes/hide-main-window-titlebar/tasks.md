## 1. 窗口外观

- [x] 1.1 `src-tauri/tauri.conf.json` 的 `main` 窗口配置增加 `"titleBarStyle": "Overlay"` 与 `"hiddenTitle": true`，`decorations` 保持默认（true）
- [x] 1.2 `src-tauri/capabilities/default.json` 增加 `"core:window:allow-start-dragging"`（`data-tauri-drag-region` 走 IPC 调 `start_dragging`，不在 `core:default` 内）

## 2. 顶部拖拽条

- [x] 2.1 `src/windows/MainWindow.vue`：`.shell` 改为纵向布局，最上插入 `.titlebar`（`flex: 0 0 auto; height: 32px`），原来的三栏移进 `.body`（`display: flex; flex: 1; min-height: 0`）
- [x] 2.2 `.titlebar` 左段宽度绑定 `settings.sidebarWidth`、底色 `--bg-sidebar`、右缘 `1px solid var(--border)`；右段 `flex: 1`、底色 `--bg`
- [x] 2.3 两段都加 `data-tauri-drag-region`
- [x] 2.4 对话框仍留在 `.shell` 下（`position: fixed; inset: 0` 覆盖整窗，不需要挪动）

## 3. 核对与收口

- [x] 3.1 dev 机截图核对：红绿灯不与拖拽条内文字/控件重叠；拖拽条左段与侧栏的分隔线对齐（偏差则只调 `.titlebar` 高度）——实测红绿灯 y 8–20 落在 32px 条内、分隔线两处均在 299pt，见 `verification.md` §2
- [x] 3.2 `pnpm vitest run`、`npx vue-tsc --noEmit`、`npx vite build`、`cargo build`、`cargo test --lib`、`openspec validate --all --strict`
- [x] 3.3 写 `verification.md`，`CHANGELOG.md` 在 0.6.0 下补条目

## 4. 真机确认（人做，未做即未通过）

> 本轮只做到“开发机截图 + 自动化回归”，下面三项未经用户实机验证，保持未勾选。

- [ ] 4.1 唤出主窗口：顶部没有标题文字与标题栏底色，界面从窗口上沿开始
- [ ] 4.2 拖顶部空条能移动窗口；双击空条能缩放
- [ ] 4.3 红绿灯仍在且行为不变（红=隐藏、黄=最小化、绿=全屏）
