# Notes

macOS local-first Markdown notes + code-snippet manager. Tauri 2 (Rust) + Vue 3 + SQLite.

产品范围与验收标准以 OpenSpec 为准：[`openspec/changes/v1-notes-manager/`](openspec/changes/v1-notes-manager/)（proposal / specs / design / tasks）。

```bash
openspec status --change v1-notes-manager   # 产物与任务进度
openspec validate v1-notes-manager --strict # 规格校验
```

`docs/ARCHITECTURE.md` 是设计参考（代码分层与约定），不作为规格来源。
发行与安装步骤见 [`docs/RELEASE.md`](docs/RELEASE.md)，版本变更见 [`CHANGELOG.md`](CHANGELOG.md)。

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
cd src-tauri && cargo test                                  # 单元 + 集成
cd src-tauri && cargo test --test database --test library --test failures --test recovery -- --test-threads=1
```

打包：`pnpm tauri build`（本机架构）或
`pnpm tauri build --target universal-apple-darwin`（需先
`rustup target add aarch64-apple-darwin`）。未做签名 / notarization，产物路径与
Gatekeeper 放行方式见 `docs/RELEASE.md`。

性能与驻留：

```bash
scripts/perf-check.sh startup   # 进程 / 库打开 / WebKit 内容进程 三个里程碑
scripts/perf-check.sh soak 24   # 24h 空闲采样（CPU% / RSS）
```

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
笔记库打开/新建/迁移/备份与占用统计、库失效回退提示、云同步目录提示、锁冲突错误类型、
开机启动开关、FTS5 搜索（中文逐字索引）、
预览安全（html:false + 协议白名单）与 Shiki 按需语法高亮、CodeMirror 主题联动
（编辑器与预览随深浅色同步换色）、全站文字对比度守门（`src/styles/tokens.test.ts`
对文本令牌 × 表面 × 主题 × 强调色断言 WCAG AA）。

测试与演练：数据层压测（1,000 条）、失败演练（只读目录 / 损坏文件 / 迁移中断 /
版本过新）、崩溃恢复（真实子进程硬退出）、备份还原往返、左栏组件测试、
1,000 条分页无重复无遗漏。

未完成：Developer ID 签名与公证、24 小时驻留实测（脚本已就绪）、端到端 UI 冒烟与
macOS 实机交互验收（快捷键连按、多显示器、Spaces、菜单栏观感均需真机人工确认）。
剩余工作见 `openspec/changes/v1-notes-manager/tasks.md` 未勾选项。
