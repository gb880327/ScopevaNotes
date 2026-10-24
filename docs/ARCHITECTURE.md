# ARCHITECTURE

## 分层

```
Vue (src/)                      Rust (src-tauri/src/)
──────────────────────────      ────────────────────────────────────────────
components / features           commands/*        IPC 边界，async，spawn_blocking
  ↓                             services/*        笔记库路径、配置、迁移编排
stores/*  (Pinia)               repositories/*    SQL（notes / tags）
  ↓                             database/*        连接、迁移、FTS 投影、备份
services/commands.ts   ──────▶  models/*          DTO + AppError
```

- 前端不执行 SQL，也不直接触碰文件系统；唯一 IPC 入口是 `src/services/commands.ts`。
- DTO 在 `src-tauri/src/models/*.rs` 与 `src/services/types.ts` 成对维护，字段为 camelCase。
- 错误统一为 `AppError { kind, message }`；`commands.ts` 把 invoke 的 rejection 规范化为 `CommandError`。

## 数据库

- 一条 `rusqlite::Connection`，由 `Database` 的 `Mutex` 守护；所有调用经
  `commands::blocking(|| db.with_conn(..))` 进入 `spawn_blocking`，不阻塞 UI 线程。
- 迁移在 `database/migrations.rs` 顺序执行，每个版本一个事务；未知的更高版本直接拒绝写入。
- 时间戳统一 Unix 毫秒 UTC；ID 使用 UUIDv7。
- `notes_fts` 由 `search_index::reindex_note` 在同一事务内重建；创建、更新、软删除、
  恢复、标签改名/删除都会触发。
- 备份使用 `VACUUM INTO`，目标已存在则报错，不做覆盖。

## 窗口与菜单栏

`WindowManager` 是唯一 show/hide 入口（快捷键、tray、IPC、关闭按钮都走它），
内部 `Mutex` 串行化请求，`visible` 只在切换成功后更新。

```
HIDDEN --toggle--> SHOWING --show+focus+alwaysOnTop--> VISIBLE
VISIBLE --toggle / close button--> HIDDEN
ANY --tray Quit--> exit(0)
```

`show` 时：恢复上次逻辑尺寸 → clamp 到鼠标所在显示器 → 居中 → 置顶 → 聚焦。
每次显示都重新居中，只持久化尺寸不持久化位置。

## 自动保存

`composables/useAutosave.ts` 提供串行化 + 防抖队列；`stores/notes.ts` 持有队列。
保存前比较 `expectedUpdatedAt`（乐观并发），落库后若期间又有输入，只更新
`updatedAt` 而不覆盖本地草稿。切换笔记、隐藏窗口、开关笔记库前都必须 `flush()`。
