# IMPLEMENTATION_PLAN

**规格已迁移到 OpenSpec，本目录不再承载规划。**

唯一验收基线：

- 变更：`openspec/changes/v1-notes-manager/`
  - `proposal.md` — 为什么做、做什么、影响面
  - `specs/*/spec.md` — 7 个能力的可验收需求与场景
  - `design.md` — 技术取向与权衡
  - `tasks.md` — 按能力分组的任务清单（勾选状态反映实际进度）
- 命令：`openspec status --change v1-notes-manager`、`openspec validate v1-notes-manager --strict`

历史输入（只读参考，不再更新）：

- [`macos-notes-manager-v1-implementation-plan.md`](./macos-notes-manager-v1-implementation-plan.md) — 最初的完整实施方案，已拆解进上述规格与任务
- [`ARCHITECTURE.md`](./ARCHITECTURE.md)、[`DECISIONS.md`](./DECISIONS.md)、[`UI_SPEC.md`](./UI_SPEC.md)、[`TEST_PLAN.md`](./TEST_PLAN.md)、[`POC_FINDINGS.md`](./POC_FINDINGS.md)

新需求或范围变更走新的 OpenSpec change，不再直接改这些文档。
