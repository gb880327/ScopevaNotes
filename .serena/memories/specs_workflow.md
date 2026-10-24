# OpenSpec workflow (acceptance baseline)

- Change `v1-notes-manager` (schema `spec-driven`) in `openspec/changes/v1-notes-manager/` is the **only**
  source of truth; `openspec/specs/` is still empty until the change is archived.
- Artifacts: `proposal.md`, `design.md` (decision log D1-D15), `tasks.md` (progress ledger), and capability specs
  under `specs/`: `notes-core`, `search`, `library`, `markdown-editing`, `macos-shell`, `appearance`, `release-readiness`.
- Requirement blocks use `### Requirement:` + `#### Scenario:` with WHEN/THEN bullets; the strict validator rejects
  other shapes or scenarios without WHEN/THEN.
- Behaviour change = edit specs + `tasks.md` in the same commit as the code; checkbox counts are the reported
  progress (`openspec status --change v1-notes-manager --json` -> `{total, complete, remaining}`).
- `design.md` also records rejected alternatives; when reality diverges from a decision, rewrite the decision
  (e.g. D6 was superseded by D15 for search) instead of leaving stale prose.
- Chinese UI/product wording lives in the specs too; keep spec language Chinese when editing requirements.
- Validate with `openspec validate v1-notes-manager --strict` after **every** artifact edit.
- Historical `docs/*` design documents were migrated into this change and deleted; `docs/ARCHITECTURE.md` and
  `docs/logo.png` remain.
