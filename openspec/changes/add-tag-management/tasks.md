## 1. 后端（无 schema 变更）

- [ ] 1.1 复核删除确认用的计数来源：`tag_repo::list_with_counts` 的 `noteCount` 已排除软删除笔记，直接复用，不新增查询。验证：在 `tag_repo::list_with_counts` 的注释里写明该复用关系，`cargo test --test database` 通过（已有"计数排除已删除笔记"断言）。
- [ ] 1.2 `delete_tag` 在 1,000 条关联笔记下的耗时基准（`--ignored`，release）。验证：`cargo test --release --test tag_bench -- --ignored --nocapture` 输出一条 p50/max 记录并写进 `verification.md`；若 > 1 s，在 design 的 Risks 里补实测与后续动作。
- [ ] 1.3 四个命令的 Rust 签名与 `src/services/commands.ts` 一一对齐（`list_tags` / `create_tag` / `rename_tag` / `delete_tag`，字段名 camelCase）。验证：`pnpm vue-tsc --noEmit` 通过，人工比对签名。

## 2. 前端 store 与写路径

- [ ] 2.1 `stores/notes.ts` 增加 `renameTag(id, name)` / `deleteTag(id)` / `setTagColor(id, color)`，失败抛出可读错误且不清空现有 `tags`。验证：`stores/notes.test.ts` 新增用例——重命名成功后 `tags` 与选中笔记的标签同步更新；失败后 `tags` 保持原值。
- [ ] 2.2 `stores/notes.ts` 增加 `attachTag(tagId)` / `detachTag(tagId)` / `createAndAttachTag(name)`，最终都走 `updateNote({ tagIds })` 并受 `editVersion` 守卫。验证：`stores/notes.test.ts` 断言以服务端回包为准，旧回包不覆盖新状态。
- [ ] 2.3 `createAndAttachTag` 的两步失败语义：标签创建成功但挂载失败时抛带 `tagCreated: true` 的错误，由界面提示"标签已创建但未挂上"。验证：`stores/notes.test.ts` mock 第二次调用失败，断言错误标记存在且 `tags` 里能查到新标签。
- [ ] 2.4 标签写操作成功后刷新 `list_tags`（计数同步），不做乐观加减。验证：`stores/notes.test.ts` 断言 `renameTag` / `deleteTag` / `attachTag` / `detachTag` 之后 `refreshTags()` 被调用。

## 3. 侧栏就地编辑（TagList）

- [ ] 3.1 `TagList.vue` 每行结构改为：色点 + 名称 + 计数，行尾 hover 出现「重命名」「删除」两个图标按钮（`aria-label` 与 `data-testid` 齐全）；点击名称区仍然只做过滤。验证：`TagList.test.ts`（新建或扩展）——点击名称区调用过滤回调且不进入编辑态；hover 后两个按钮存在。
- [ ] 3.2 重命名就地：点击重命名 icon 后整行变成输入框（复用 `TagEditor`），预填当前名称并全选；回车提交、Esc 取消、失焦取消；提交成功退出编辑态，冲突或空名则留在输入框并就地显示错误。验证：`TagList.test.ts` 四条用例（回车提交调用 `renameTag`、Esc 不调用、冲突错误可见且行仍可编辑、空名不调用）。
- [ ] 3.3 删除行内二次确认：第一次点击把行内换成"将影响 N 条笔记 —— 删除 / 取消"，二次确认才调用 `deleteTag`，取消恢复原行。验证：`TagList.test.ts` 断言取消时 `deleteTag` 未被调用、确认时以正确 id 调用一次，且文案包含 `noteCount`。
- [ ] 3.4 改色：点击色点展开 8 色调色板（取自设计令牌），选中后调用 `setTagColor` 并收起；刷新后色点仍是该色。验证：`TagList.test.ts` 断言 `setTagColor` 收到预期色值；`tokens.test.ts` 全绿。
- [ ] 3.5 空态与新建：没有标签时显示"还没有标签"提示（不只是空列表）；`＋` 新建行为保持现状但错误内联显示。验证：`TagList.test.ts` 空态用例 + 新建冲突用例。

## 4. 笔记标签编辑（NoteHeader）

- [ ] 4.1 `NoteHeader.vue` 的 `.meta` 行改为标签 chip：每个 chip 带 `×`（摘除），行尾 `＋`（打开选择器）；标签多时横向滚动不换行。验证：`NoteHeader.test.ts` 渲染 2 个标签并断言 2 个摘除按钮存在。
- [ ] 4.2 选择器列出未挂载的标签并支持输入新名称（回车即建即挂）；标签编辑不依赖 `viewMode`（只读模式下也能改）。验证：`NoteHeader.test.ts` —— 选择已有标签调用 `attachTag`；输入新名称调用 `createAndAttachTag`；只读模式下点击 `＋` 仍能打开选择器。
- [ ] 4.3 摘除与挂载失败时保留界面状态并显示错误（不静默回滚）。验证：`NoteHeader.test.ts` mock 失败，断言错误文本出现且 chip 仍在。

## 5. 验证与文档

- [ ] 5.1 端到端手工验证（待人工）：新建标签 → 侧栏重命名 → 改色 → 挂到笔记 → 从笔记摘除 → 侧栏删除（确认文案核对受影响条数），每步核对侧栏计数、搜索结果与笔记头部。验证：步骤记录与截图写进 `verification.md` 对应场景。
- [ ] 5.2 规格对照：2 条 requirement / 9 个 scenario 逐条映射证据写进 `verification.md`（三态：自动 / 人工已验 / 待人工）。验证：`openspec validate add-tag-management --strict` 通过且矩阵无空洞。
- [ ] 5.3 文档：`CHANGELOG.md` 增条目、`README.md` 功能清单补一行、`docs/RELEASE.md` 补"标签维护在侧栏就地完成"两行。验证：三处存在且与 spec 表述不冲突。
- [ ] 5.4 全量回归：`pnpm vue-tsc --noEmit`、`pnpm vitest run`、`pnpm vite build`、`cargo test`（集成测试加 `--test-threads=1`）。验证：四条命令全部通过。
