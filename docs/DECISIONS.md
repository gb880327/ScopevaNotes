# DECISIONS

按时间顺序记录与计划文档不一致、或计划留给实现决定的事项。

## D-001 分层收敛：不设独立 service 层（除笔记库）

`services/note_service.rs`、`services/search_service.rs` 暂未创建。当前 notes/tags
的业务规则（事务、乐观并发、FTS 投影、标签规范化）已完整落在 `repositories/*`，
再包一层纯转发没有收益。待 M4 搜索排序/短词回退需要多仓库编排时再引入 service 层。
`services/library_service.rs` 保留：路径校验、配置读写、笔记库信息是真实的业务逻辑。

## D-002 数据库连接：单连接 + Mutex + spawn_blocking

计划要求“专用数据库线程/队列，或受控连接池 + spawn_blocking”。V1 数据量小、写入
串行，采用单 `Connection` + `Mutex`，所有调用经 `spawn_blocking`，满足不阻塞 UI 的
要求。若 M4 搜索压测出现锁竞争，再迁移到专用 DB 线程。

## D-003 搜索：先用 LIKE，FTS5 表已就绪

`list_notes` 目前用 `title/content LIKE`（已转义 `% _ \`）实现子串搜索，中文短词
天然可用。`notes_fts` 表和 `reindex_note` 已实现并在所有写路径维护，M4 只需把查询
切到 `MATCH`（`to_match_query` 已备好）并补排序/分页基准。

## D-004 启动失败回退

自定义笔记库路径不存在时，`library_service::resolve` 返回错误。若直接失败，用户将
看不到任何窗口。当前实现：回退到默认库并 `eprintln!` 记录，不覆盖 config.json。
M1/M5 需要把该错误通过状态字段暴露到 UI（当前是已知缺口，见 POC_FINDINGS）。

## D-005 关闭窗口不退出 + 启动可见性

窗口按计划配置为 `visible: false`，但首次启动且尚无 onboarding 流程时在 `setup`
中主动 `show`，否则开发与首次运行都是“无界面”。M2 引入引导页后改为默认隐藏。

## D-006 未实现独立 Snippet 类型 / Quick Panel / 第三栏

按计划 §0 执行，无代码。

## D-007 菜单栏图标暂用应用图标

`icons/` 由 `pnpm tauri icon` 生成的占位图。macOS 发布前必须替换为单色 template
icon（32×32 @2x，`isTemplate = true`），当前 `icon_as_template(true)` 只是占位行为。

## D-008 `move_library` 不删除旧库

迁移 = `VACUUM INTO` 生成一致性快照 → 打开新库 → 原子写 config。旧文件保留供恢复，
避免失败时丢数据。UI 需在 M5 提示用户手动清理。

## D-009 SearchResults 组件合并

计划目录含 `features/search/SearchResults.vue`，但双栏布局下搜索结果就是笔记列表，
独立组件只会重复列表逻辑。搜索状态由 `SearchInput` + `NoteList` 承担。

## D-010 类型检查与测试基线

`typescript@5.9`（非 7.x）、`vue-tsc@3`、`vite@8`、`tailwindcss@4`、`rusqlite@0.40`、
`tauri@2`。`Cargo.lock` / `pnpm-lock.yaml` 提交入库，作为实际锁定版本依据。
