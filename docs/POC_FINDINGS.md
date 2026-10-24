# POC_FINDINGS (M0)

本文件只记录**已验证**与**未验证**的事实。未在真机执行的项目不得写成通过。

## 已验证（本机，Intel macOS 15.8.1）

- [x] Tauri 2 + Vue 3 + Vite 工程可编译（`cargo check`、`pnpm vue-tsc`、`vite build`）。
- [x] SQLite（bundled rusqlite）建库、迁移、CRUD、软删除/恢复、FTS 投影集成测试通过。
- [x] 全局快捷键插件、单实例插件、tray、autostart、dialog、store、clipboard、opener
      依赖均可编译链接；快捷键注册失败不会中断启动（回退为日志）。
- [x] 窗口状态机（HIDDEN ↔ VISIBLE）与关闭按钮 → 隐藏的代码路径已接好；
      居中算法有单元测试（含负坐标副屏、超大窗口 clamp）。
- [x] 打包图标生成（`pnpm tauri icon`）。

## 未验证（必须实机执行后才能勾选）

- [ ] 快捷键连按 100 次不出现焦点异常/重复窗口。
- [ ] 菜单栏左键切换、右键菜单项逐项可用；左右键区分在目标 Tauri 版本是否可靠。
- [ ] 多显示器居中、外接屏断开后窗口不会出现在屏幕外。
- [ ] Spaces / 原生全屏 / Stage Manager 下 always-on-top 的实际表现。
- [ ] 关闭隐藏后 Dock 行为、⌘Q 与 tray 退出的差异。
- [ ] 开机启动（LaunchAgent）在目标系统上的实际行为。
- [ ] `pnpm tauri dev` 首次运行的 WebView 加载与快捷键注册顺序。
- [ ] 白底图标在深色菜单栏下的可见性（需要 template icon）。

## 已知缺口

1. 自定义笔记库路径失效时只回退并打日志，UI 无提示（D-004）。
2. 搜索仍是 LIKE 子串，未切 FTS5 MATCH（D-003）。
3. 代码高亮未接入 Shiki，当前为纯文本 `<pre>`。
4. 回收站无入口（`restore_note` 命令已实现，UI 未暴露）。
5. 全局快捷键不可配置（固定 `CommandOrControl+Shift+Space`）。
6. `beforeunload`/窗口隐藏时的 flush 仅在 UI 操作路径覆盖，进程被强杀仍可能丢最后
   一次未 flush 的编辑（M5 需接 Tauri 退出钩子）。
