# OpenSpec workflow (acceptance baseline)

- **`openspec/specs/` is the living baseline** (8 capabilities: `notes-core`, `search`, `library`,
  `markdown-editing`, `markdown-import`, `macos-shell`, `appearance`, `release-readiness`). Two changes were
  archived into it on 2026-10-24: `v1-notes-manager` (70/70) and `add-markdown-import` (24/26, two manual
  verification tasks left open on purpose). History, decision logs and `verification.md` stay readable under
  `openspec/changes/archive/<date>-<name>/`.
- New work starts as a change again: `openspec new change <name>` -> proposal / delta specs / design / tasks,
  `openspec validate <name> --strict` after **every** artifact edit, `openspec archive <name> -y` at the end
  (the archive command merges the delta into `openspec/specs/`, it also warns on incomplete tasks but does not block).
- Delta specs use `## ADDED/MODIFIED/REMOVED/RENAMED Requirements` heads plus `### Requirement:` and
  `#### Scenario:` with WHEN/THEN bullets; the strict validator rejects other shapes, scenarios without
  WHEN/THEN, and a `## Purpose` shorter than 50 characters (that last one bites when a carried purpose is terse).
- Behaviour change = edit the baseline spec (or a change's delta) in the same commit as the code; `tasks.md`
  checkboxes are the reported progress (`openspec status --change <name> --json`).
- `design.md` also records rejected alternatives; when reality diverges from a decision, rewrite the decision
  (e.g. D6 was superseded by D15 for search; D9 was narrowed by D14 for bulk import) instead of leaving stale prose.
- Chinese UI/product wording lives in the specs too; keep spec language Chinese when editing requirements.
- Historical `docs/*` design documents were migrated into the changes and deleted; `docs/ARCHITECTURE.md` and
  `docs/logo.png` remain (code-level reference only, never the spec source).
