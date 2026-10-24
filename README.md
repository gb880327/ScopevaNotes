# Notes

macOS local-first Markdown notes + code-snippet manager. Tauri 2 (Rust) + Vue 3 + SQLite.

产品范围与验收标准见 [`docs/macos-notes-manager-v1-implementation-plan.md`](docs/macos-notes-manager-v1-implementation-plan.md)。
实现取舍见 [`docs/DECISIONS.md`](docs/DECISIONS.md)。

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
双栏 UI + CodeMirror 编辑 + Markdown 预览 + 代码块复制 + 自动保存、菜单栏、
全局快捷键、关闭隐藏、窗口居中/置顶/尺寸记忆、笔记库打开/新建/迁移/备份。

未完成：FTS 搜索（当前为 LIKE 子串）、Shiki 高亮、回收站视图、缩短快捷键自定义 UI、
签名分发、macOS 实机交互验收。详见 `docs/POC_FINDINGS.md` 与 `docs/TEST_PLAN.md`。
