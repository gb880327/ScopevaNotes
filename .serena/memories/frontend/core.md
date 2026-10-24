# Vue side

## Structure
- `windows/MainWindow.vue` = shell: hydrates stores, wires `components/layout/` (Sidebar 280px resizable,
  ContentPane), owns global keyboard handling and the quit handshake.
- `stores/`: `notes.ts` (list/detail/trash/save status/conflict), `ui.ts` (dialogs, panels, viewMode),
  `settings.ts` (prefs + `systemDark` + `resolvedTheme` computed - the single source for theme-dependent
  rendering).
- `composables/`: `useAutosave` (debounce queue + `discard()`), `useTheme` (writes `data-theme`/`data-accent`
  from the store and feeds `prefers-color-scheme` into `settings.systemDark`), `useKeyboard`.
- `features/<domain>/`: notes (list, editor, preview, code block, trash), search, tags, settings.
- `services/`: `commands.ts` (IPC), `types.ts` (DTOs), `shortcut.ts`, `format.ts`, `clipboard.ts`.

## Boot sequence
`main.ts` mounts App -> `MainWindow` calls `settings.hydrate()` then `settings.applyShortcut()`;
if the stored accelerator is taken, it falls back to the default and reports `shortcutError`.
Rust may emit `ui://quit-requested` at any time; handler = `notes.flush()` -> on success `api.requestQuit()`,
otherwise an error `message()` and **no** quit.

## Settings persistence
- Tauri Store file `settings.json`, key `appearance`; `PersistedSettings` =
  `{theme, accent, editorFontSize, sidebarWidth, shortcut}`.
- `localStorage['notes.settings']` is a fallback for browser-only dev and must stay in sync on write.

## Preview pipeline (spec: markdown-editing, design D16)
- `features/notes/fences.ts` splits the note into prose/code **before** rendering: line scanner, closing
  fence needs the same char and >= the opening length, backtick info strings cannot contain backticks,
  unterminated fence runs to EOF and stays copyable. Prose -> `markdown.ts`, code -> `CodeBlock.vue`.
- `markdown.ts`: markdown-it with **`html: false`** plus a scheme **allowlist** (`http`/`https`/`mailto`/
  relative; control chars stripped first, `//host` rejected). Never swap it for a blacklist.
- `NotePreview.vue` `v-html`s only markdown-it output; its click handler always `preventDefault()`s and
  forwards only http(s) to the opener, so the webview can never be navigated away.
- `highlight.ts`: `shiki/core` + JS regex engine, 42 languages via per-language dynamic `import()` (own
  chunk each) plus aliases (`ts`, `sh`, `yml`, `c++`...); unknown/blank/oversized (>20000 chars)/failed
  grammar -> `null` -> plain text. Light = `github-light-high-contrast`, dark = `github-dark` (plain
  `github-light` measured 3.2:1 on the code background). `CodeBlock` re-highlights from
  `settings.resolvedTheme`.
- CodeMirror theming lives in `features/notes/editorTheme.ts`: `editorChrome` uses design tokens only
  (gutters, caret, selection, search panel, brackets), while syntax colours sit in `EDITOR_PALETTES`
  and are swapped via a `Compartment` on `settings.resolvedTheme` (never recreate the EditorView).
  Palettes are re-tuned for `--bg` (`#ffffff` / `#1c1c1e`), not Shiki's `#24292e`; the test reads
  `styles/tokens.css` and enforces >= 4.5:1. `--text-faint` used to fail that on white (3.26:1), so the
  gutter uses `--text-muted`.

## Design tokens and contrast (task 7.4)
- `--accent` is **fill only** (dots, selection tint, focus borders). Text, icons and primary button
  backgrounds use `--accent-text`, with `--accent-contrast` as the label colour on that fill. One value
  cannot be both readable-as-text (deep on white) and vivid-as-fill, and dark theme needs the inverse
  shade, so every accent has a light and a dark `--accent-text` (`[data-theme='dark'][data-accent=…]`).
- Current values: light `--text-muted #5f5f66`, `--text-faint #68686e`, `--danger #cc0013`,
  `--selected: rgba(0,0,0,0.05)`; dark `--text-muted #b0b0b6`, `--text-faint #a3a3a9`,
  `--danger #ff7a72`. Do not lighten `--text-faint`/`--selected` back — `tokens.test.ts` fails first
  (`--selected` darkens the surface, which is what used to push secondary text under 4.5:1).
- `styles/tokens.test.ts` is the gate: it parses the CSS by selector, composites translucent surfaces
  (`--bg-input`, `--selected`, `--accent-soft`) over `--bg`, and asserts 5 text tokens × 7 surfaces ×
  2 themes × 5 accents, plus `--accent-contrast` on `--accent-text`. Change a colour there and it names
  the exact pair and ratio.
- Accepted, deliberately unasserted: `--accent` fills as non-text UI (green/orange dots are 2:1 on
  white, every dot sits next to a text label), `--border*` (Apple-style low-contrast separators) and
  user-chosen tag dot colours (stored in the DB, not a token).

## Autosave + conflicts
- `useAutosave` debounces edits; `saveStatus` is `idle`/`saving`/`saved`/`error`/`conflict`.
- A stale `updatedAt` response sets `conflict` + `conflictNoteId`; resolution is either
  `resolveConflictReload()` (discard local + refetch) or `resolveConflictOverwrite()` (`force: true`).
- New write paths must reuse `update_note` with `expectedUpdatedAt`/`force`, not a second SQL branch.

## UI conventions
- Tailwind 4 utility classes inline + tokens in `styles/tokens.css` (theme/accent/font-size are CSS vars).
- Dialogs/menus from `reka-ui`; transient feedback via the store-driven `message()` toast.
- Component tests sit next to the component (`*.test.ts`), mount with a fresh `createPinia()` and mock
  `@/services/clipboard`; jsdom runs the real Shiki, so highlighting is asserted on rendered spans.

## List request ordering (hard rule)
Every list request in `stores/notes.ts` takes a ticket from `listSeq` and drops its own result if a
newer request has started (`refresh()` and `loadMore()` both do this). Slow search responses must never
replace newer items — the spec scenario "过期响应被丢弃" is covered by `stores/notes.test.ts`.
`editVersion` does the same job for save responses: an older `updateNote` reply may refresh
`updatedAt` but must not overwrite text typed while the request was in flight.

## Test layout
`*.test.ts` next to the module (jsdom via `vitest.config.ts`), IPC always mocked at
`@/services/commands`. Store tests set `setActivePinia(createPinia())` per case; the desktop settings
test mocks `@tauri-apps/plugin-store` with an in-memory map instead of touching Tauri.

## Tag editing write path
- Tag/relation edits reuse the single write path: `commitNoteTags` mutates `selected`, bumps `editVersion`, `queue.discard()`, then `saveNow()` → `update_note({ tagIds })`. Never add `add_note_tag`/`remove_note_tag` commands (second write path + second index maintenance).
- `createAndAttachTag` is deliberately two steps (create then attach). Attach failure throws `TagAttachError` (`tagCreated = true`) so the UI says "tag created but not attached"; the tag stays in the sidebar for a retry.
- Successful tag writes re-query `list_tags` (`refreshTags()`) instead of optimistic ±1 on counts.
- Popovers inside the sidebar/`.meta` **must not** be `position: absolute`: `.scroll` (`overflow-y: auto`) and `.meta` (`overflow-x: auto`) clip them to one line. Render them in flow (palette = wrapped second line of the row, NoteHeader picker = sibling below the chip row).

## Tag filter
- `activeTagIds` is an array on the transport but a **single** value in the UI: `toggleTagFilter(id)` sets `[id]` or `[]`, never appends. Multi-tag filtering is not a product feature (baseline spec `notes-core` says so) — do not "restore" additive selection.

## Trash UI
- `stores/notes.ts` keeps `deleted` (list, capped) and `deletedCount` (whole trash, from `count_deleted_notes`). `purgeNote` / `emptyTrash` run the command first, then re-read trash + list + tag counts; they rethrow after setting `error`, so a failure leaves the UI untouched (no optimistic removal on a destructive action).
- `TrashList.vue` confirmations (per-row purge and clear-trash) are in-flow panels (`flex-basis: 100%` inside a `flex-wrap: wrap` row), never absolute — `.trash` scrolls and would clip a popover. Esc closes them via a `document` keydown listener that is attached/detached by a `watch` and removed in `onUnmounted`.
- Row tests use `data-testid`: `purge-<id>`, `purge-confirm`, `confirm-purge`, `cancel-purge`, `empty-trash`, `clear-confirm`, `confirm-empty-trash`, `cancel-empty-trash`.
