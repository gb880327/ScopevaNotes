# TEST_PLAN

## 已落地

| 层 | 位置 | 覆盖 |
| --- | --- | --- |
| Rust 单元 | `src/platform/macos/monitor.rs` | 主屏/负坐标屏居中、超大窗口 clamp |
| Rust 集成 | `src-tauri/tests/database.rs` | 迁移建库、创建/查询/更新、标签规范化与唯一性、乐观并发冲突、软删除与恢复、FTS 投影增删 |
| 前端单元 | `src/features/notes/fences.test.ts` | 代码围栏切分、语言识别、无围栏/未闭合/`~~~` |
| 类型/构建 | `pnpm vue-tsc --noEmit`、`pnpm vite build` | 前端类型与打包 |
| 编译 | `cargo check`、`cargo test` | Rust |

运行：

```bash
cd src-tauri && cargo test && cargo check && cd ..
pnpm vue-tsc --noEmit && pnpm vitest run && pnpm vite build
```

## 待补（按里程碑）

- M0 实机：快捷键连按 100 次、外接屏断开、Spaces/全屏/Stage Manager、置顶焦点、
  单实例、菜单栏左右键、开机启动。**均未在真机验证，不得标记为通过。**
- M1：1,000 条随机笔记 CRUD 压测；迁移失败回滚；写入中关窗不丢数据；备份/还原往返；
  损坏文件与只读目录；schema 版本过新拒绝写入。
- M2：组件测试（左栏过滤/滚动、选中态、空态、键盘访问）、窗口缩放不溢出。
- M3：Bash/TS/SQL/JSON 四类代码块渲染与复制内容逐字比对；恶意 HTML/`javascript:` 链接。
- M4：10,000 条中英混合数据检索 P95 < 100ms；快速输入不显示过期结果；深浅色跟随系统。
- M5：24h 驻留内存/CPU；冷热启动耗时；Intel 与 Apple Silicon 构建。

## 数据安全用例

- 恶意 Markdown（`<script>`、`<img onerror>`、`[x](javascript:alert(1))`）
- 超长代码围栏、未闭合围栏
- 强制结束/断电后重启（WAL 恢复）
- 指向不存在路径 / 只读路径的笔记库
