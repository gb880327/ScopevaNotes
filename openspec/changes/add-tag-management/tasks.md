## 1. 后端补齐（无 schema 变更）

- [ ] 1.1 `note_repo`（或 `tag_repo`）加只读查询：给定 `tag_id` 返回受影响的可见笔记数，供删除确认文案使用；若直接复用 `list_with_counts` 的 `noteCount` 则本任务改为在 `tag_repo` 注释里写明该复用关系。验证：`cargo test --test database` 通过，且新增断言确认已软删除笔记不计入。
- [ ] 1.2 冒烟确认 `delete_tag` 在 N=1,000 条关联下的耗时（`--ignored` 基准，release），把数字写进 `verification.md`。验证：`cargo test --release --test tag_bench -- --ignored --nocapture` 输出一条 p50/最大值记录；若 > 1 s 在 design 的 Risks 里追加实测与后续动作。
- [ ] 1.3 `commands/notes.rs` / `commands/tags.rs` 的取消注册与参数名复核：`update_note` 的 `tagIds`、`rename_tag`、`delete_tag` 在 `src/services/commands.ts` 里一一对应且 DTO 字段名一致。验证：`pnpm vue-tsc --noEmit` 通过，人工比对四个命令的 Rust 签名与 TS 类型。

## 2. 前端 store 与写路径

- [ ] 2.1 `stores/notes.ts` 增加 `renameTag(id, name)` / `deleteTag(id)` / `setTagColor(id, color)`，失败时抛出可读错误且不清空现有 `tags`。验证：`stores/notes.test.ts` 新增用例——重命名成功后 `tags` 与选中笔记的标签同步更新；失败后 `tags` 保持原值。
- [ ] 2.2 `stores/notes.ts` 增加 `attachTag(tagId)` / `detachTag(tagId)` / `createAndAttachTag(name)`，全部最终走 `updateNote({ tagIds })` 并走 `editVersion` 守卫。验证：`stores/notes.test.ts` 断言乐观结果以服务端回包为准，且旧回包不覆盖新状态。
- [ ] 2.3 `createAndAttachTag` 的两步失败语义：标签创建成功但挂载失败时，抛出带 `tagCreated: true` 的错误，由界面提示"标签已创建但未挂上"。验证：`stores/notes.test.ts` 用例 mock 第二次调用失败，断言错误标记存在且 `tags` 里能查到新标签。

## 3. 标签管理界面

- [ ] 3.1 新建 `src/features/tags/TagManager.vue`：列出全部标签（颜色点、名称、可见笔记数），行内提供重命名输入、调色板与删除按钮；空标签列表时显示空态文案。验证：新建 `TagManager.test.ts`——渲染 3 个标签并断言名称与计数；空列表断言空态。
- [ ] 3.2 删除走内联确认：第一次点击显示"将影响 N 条笔记"与确认/取消，确认后才调用 `deleteTag`，取消则不发命令。验证：`TagManager.test.ts` 用例断言取消时 `deleteTag` 未被调用，确认时以正确 `id` 调用一次。
- [ ] 3.3 重命名冲突与空名在界面内联报错（错误文本来自命令返回），并把输入恢复为可编辑状态。验证：`TagManager.test.ts` 两条用例：冲突错误可见且行仍是原名；空名不调用 `renameTag`。
- [ ] 3.4 调色板从既有设计令牌里的强调色集合取值，写入 `tags.color`；刷新后颜色点仍然是该色。验证：`TagManager.test.ts` 断言 `setTagColor` 收到预期色值；`tokens.test.ts` 仍全绿（新增色值若来自令牌则不需要改断言）。
- [ ] 3.5 `SettingsDialog.vue` 增加「标签」section（标题 + 说明 + `TagManager`），放在「笔记库」之后、「迁移」之前。验证：`SettingsDialog` 相关测试或新增断言——打开设置能看到标签 section，且 `TagManager` 被渲染一次。

## 4. 笔记标签编辑

- [ ] 4.1 `NoteHeader.vue` 的 `.meta` 行改为标签 chip：每个 chip 带 `×`（摘除）、行尾 `＋`（打开选择器）。验证：`NoteHeader.test.ts` 渲染 2 个标签并断言 2 个摘除按钮存在。
- [ ] 4.2 选择器列出未挂载的标签并支持输入新名称（回车即建即挂）；只读模式与编辑模式行为一致（标签编辑不依赖 `viewMode`）。验证：`NoteHeader.test.ts` —— 选择已有标签调用 `attachTag`；输入新名称调用 `createAndAttachTag`；只读模式下点击 `＋` 也能打开选择器。
- [ ] 4.3 摘除与挂载失败时保留界面状态并显示错误（不静默回滚）。验证：`NoteHeader.test.ts` mock 失败，断言错误文本出现且 chip 仍在。
- [ ] 4.4 标签变更后 `list_tags` 刷新（计数同步），不依赖乐观加减。验证：`stores/notes.test.ts` 断言 `attachTag`/`detachTag` 之后 `refreshTags()` 被调用。

## 5. 验证与文档

- [ ] 5.1 端到端手工验证（待人工）：新建标签 → 挂到笔记 → 重命名 → 改色 → 从笔记摘除 → 删除标签，每步核对侧栏计数、搜索结果与笔记头部。验证：步骤记录与截图写入 `verification.md` 的对应场景。
- [ ] 5.2 规格对照：把本 change 的 2 条 requirement / 8 个 scenario 逐条映射证据写进 `verification.md`（沿用三态：自动 / 人工已验 / 待人工）。验证：`openspec validate add-tag-management --strict` 通过且矩阵无空洞。
- [ ] 5.3 文档：`CHANGELOG.md` 增加条目（0.3.0 或 unreleased 段）、`README.md` 功能清单补一行、`docs/RELEASE.md` 补"标签维护"两行。验证：三处存在且不与 spec 表述冲突。
- [ ] 5.4 全量回归：`pnpm vue-tsc --noEmit`、`pnpm vitest run`、`pnpm vite build`、`cargo test`（集成测试加 `--test-threads=1`）。验证：四条命令全部通过。
