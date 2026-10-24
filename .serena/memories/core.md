# Notes (Scopeva) — project root map

macOS local-first Markdown notes + code snippets. Two halves in one repo:
- `src-tauri/` Rust crate `macos-notes` (lib `macos_notes_lib`, bin `macos-notes`).
- `src/` Vue 3 SPA, **single window** label `main`; no router, no second window.

## Product invariants (contractual, do not "improve" away)
- One entity only: `Note`. No Snippet entity, no Quick Panel, no third pane, no multi-window.
- Window 1100x720, min 760x480, `visible: false` in `tauri.conf.json`; close button hides, tray "退出 Notes" quits.
- Default global shortcut `CommandOrControl+Shift+D`, user-changeable; startup silently re-registers it.
- Startup shows no main window (menu-bar resident); a one-shot native dialog explains the shortcut once.
- Quit is two-phase: flush unsaved edits first, then a native confirm dialog.
- Bundle identifier `com.scopeva.notes`; bundle targets `app` + `dmg`, min macOS 12, unsigned.

## Where truth lives
- Acceptance baseline: `openspec/specs/` (8 capabilities). History and rationale live in the archived changes
  under `openspec/changes/archive/` (`2026-10-24-v1-notes-manager`, `2026-10-24-add-markdown-import`,
  `2026-10-24-add-tag-management`, `2026-10-24-add-trash-purge`).
  See `mem:specs_workflow`.
- `docs/ARCHITECTURE.md` is a code-level reference only, never the spec.
- Decision logs: D1-D20 in `openspec/changes/archive/2026-10-24-v1-notes-manager/design.md`,
  D1-D14 in `openspec/changes/archive/2026-10-24-add-markdown-import/design.md`,
  D1-D8 in `.../2026-10-24-add-tag-management/design.md`, D1-D6 in `.../2026-10-24-add-trash-purge/design.md`.

## Memory map
- Stack, version pins, toolchain: `mem:tech_stack`.
- Commands a human runs (dev, tests, OpenSpec): `mem:suggested_commands`.
- Gate to run before calling a task done: `mem:task_completion`.
- Code style, IPC contract, layering rules: `mem:conventions`.
- Rust side: schema, migrations, state machine, tray/quit: `mem:backend/core`.
- Vue side: stores, autosave, settings persistence, events: `mem:frontend/core`.
- Search subsystem (FTS5 + CJK + its two performance traps): `mem:search`.
- OpenSpec artifacts, checkbox ledger, validation: `mem:specs_workflow`.
