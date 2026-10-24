# Notes

macOS local-first Markdown notes + code-snippet manager. Tauri 2 (Rust) + Vue 3 + SQLite.

产品范围与验收标准以 OpenSpec 为准：[`openspec/changes/v1-notes-manager/`](openspec/changes/v1-notes-manager/)（proposal / specs / design / tasks）。

```bash
openspec status --change v1-notes-manager   # 产物与任务进度
openspec validate v1-notes-manager --strict # 规格校验
```

`docs/ARCHITECTURE.md` 是设计参考（代码分层与约定），不作为规格来源。

## 开发

```bash
pnpm install
pnpm tauri dev      # 完整应用（Rust + Vue）
pnpm dev            # 仅前端；无 Tauri 运行时，会提示需要桌面环境
```

验证：

```bash
pnpm vue-tsc --noEmit   # 类型检查
pnpm vitest run         # 前端单元测试
pnpm vite build         # 前端构建
cd src-tauri && cargo test && cargo check
```

打包：`pnpm tauri build`（未做签名 / notarization）。

## 目录

```
src/                     Vue 3 前端
  services/commands.ts   IPC 契约封装（唯一调用 invoke 的地方）
  services/types.ts      与 Rust DTO 对齐的 TS 类型
  stores/                notes / ui / settings (Pinia)
  composables/           autosave 队列、主题、快捷键
  components/layout/     双栏布局
  features/              notes / search / tags / settings
src-tauri/
  migrations/            SQL 迁移（include_str! 进二进制）
  src/database/          连接、迁移、FTS 投影、备份
  src/repositories/      notes / tags 数据访问
  src/commands/          Tauri commands
  src/services/          笔记库路径与配置
  src/platform/macos/    tray、全局快捷键、窗口状态机、显示器定位
```

## 当前状态

已完成可运行骨架：数据库迁移 + notes/tags CRUD + 软删除/恢复 + FTS 投影维护、
双栏 UI + CodeMirror 编辑 + Markdown 预览 + 代码块复制 + 自动保存、回收站与恢复、
保存冲突处理（重载 / 覆盖）、菜单栏 logo、可自定义全局快捷键（默认 ⇧⌘D）、
启动静默、退出确认（先 flush 再确认）、关闭即隐藏、窗口居中/置顶/尺寸记忆、
笔记库打开/新建/迁移/备份与占用统计、开机启动开关。

未完成：Shiki 高亮、签名分发、macOS 实机交互验收。剩余工作见 `openspec/changes/v1-notes-manager/tasks.md` 未勾选项。
