# Notes

macOS local-first Markdown notes + code-snippet manager. Tauri 2 (Rust) + Vue 3 + SQLite.

产品范围与验收标准以 OpenSpec 为准。两个 change 都已归档，**`openspec/specs/` 是当前生效的基线**
（8 个 capability：appearance、library、macos-shell、markdown-editing、markdown-import、
notes-core、release-readiness、search）；归档副本保留决策与验收记录：

- [`openspec/changes/archive/2026-10-24-v1-notes-manager/`](openspec/changes/archive/2026-10-24-v1-notes-manager/)：v1 笔记管理器（70/70，design D1–D20、`verification.md`）
- [`openspec/changes/archive/2026-10-24-add-markdown-import/`](openspec/changes/archive/2026-10-24-add-markdown-import/)：批量导入 Markdown（24/26，design D1–D14、`verification.md`）

```bash
openspec validate --all --strict               # 8 个基线规格 + 归档校验
openspec list                                   # 活跃 change（当前为空）
```

改行为时先改基线规格：新建 change（`openspec new change <name>`）或直接编辑
`openspec/specs/<capability>/spec.md`，再把代码与 `tasks.md`/`verification.md` 同步。

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
cd src-tauri && cargo test --test import -- --test-threads=1
cd src-tauri && cargo test --test database --test library --test failures --test recovery -- --test-threads=1
```

打包：`pnpm tauri build`（本机架构）或
`pnpm tauri build --target universal-apple-darwin`（需先
`rustup target add aarch64-apple-darwin`）。未做签名 / notarization，产物路径与
Gatekeeper 放行方式见 `docs/RELEASE.md`。

导入规模检查（release，1,000 / 5,000 条，打印耗时与单批最长间隔）：

```bash
cd src-tauri && cargo test --release --test import_bench -- --ignored --nocapture
```

性能与驻留：

```bash
scripts/perf-check.sh startup   # 进程出现 / 笔记库打开 两个里程碑
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
  features/              notes / search / tags / settings / import
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

批量导入 Markdown（从 Obsidian 等应用一次性迁移）：三阶段模态（选择目录 → 进度 →
报告）、只读扫描与 dry-run 统计、隐藏目录跳过并计数、UTF-8/GB18030 解码（GB 回退逐条标注）、
frontmatter 子集解析（title/tags/created）、保留文件 mtime 顺序、正文逐字保真（不解析
`[[链接]]` / `%%注释%%`）、200 条/批 + 进度事件 + 批间取消、按批次标签整批撤销（进回收站）、
报告里单列「引用了本地文件（不迁移）」计数。

测试与演练：数据层压测（1,000 条）、失败演练（只读目录 / 损坏文件 / 迁移中断 /
版本过新）、崩溃恢复（真实子进程硬退出）、备份还原往返、左栏组件测试、
1,000 条分页无重复无遗漏、导入端到端（扫描/批量/取消重试/撤销/父目录标签/坏文件隔离/
mtime 保留/正文逐字保真）。

导入的已知限制：不迁移附件（图片链接原样保留但文件不复制）、不解析 Obsidian 扩展语法、
不做同步或目录监听、同一天两次导入共用 `import-YYYY-MM-DD` 这一撤销粒度。详见
`openspec/changes/archive/2026-10-24-add-markdown-import/design.md`（D1–D14）。

v1 的交付范围：**本机自用**。tasks.md 已全部勾选（70/70），其中 macOS 实机交互验收
（快捷键连按、多显示器、Spaces、菜单栏观感）、端到端 UI 冒烟、24 小时驻留采样时长与
签名公证由用户明确接受或移出范围（见 design D20 与 `verification.md`）：这些项尚无真人
观察记录，真机出问题时先从对应场景查起。
