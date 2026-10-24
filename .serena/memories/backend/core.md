# Rust side

## Module map (`src-tauri/src/`)
- `database/`: `connection.rs` (single `Mutex<Connection>`, WAL, `PRAGMA quick_check`), `migrations.rs`,
  `search_index.rs`, `backup.rs`.
- `models/`: `note.rs`, `tag.rs` (camelCase DTOs), `error.rs` (`AppError`/`AppResult`).
- `repositories/`: `note_repo.rs`, `tag_repo.rs` - all query + validation logic.
- `services/import_service.rs`: batch Markdown import (discovery, decoding, frontmatter subset, batched write loop).
- `services/library_service.rs`: resolves library path from `config.json` in `app_config_dir()`,
  creates/opens/moves/backs up, computes `LibraryInfo` (note count + on-disk size incl. `-wal`/`-shm`).
- `commands/`: `notes`, `tags`, `libraries`, `window`, `shortcuts`, `quit`, `import`; `mod.rs` holds `blocking()`.
- `platform/macos/`: `tray.rs`, `shortcuts.rs`, `window_manager.rs`, `monitor.rs` (multi-display centring).
- `state.rs`: `AppState`; `lib.rs`: builder, plugin order, `setup`, exit interception.

## Startup (`lib.rs::run`)
- `library_service::resolve` -> on error **fallback to default library** with `eprintln!`; config.json is never rewritten (D4/D12).
- `state.manage(AppState::new(db, info, shortcuts::DEFAULT_SHORTCUT))`, then `tray::setup`, then
  `shortcuts::register(DEFAULT_SHORTCUT)` (failure must not abort startup), then `show_first_run_tip`.
- Window stays hidden: `tauri.conf.json` has `visible: false` and nothing calls `show()` at boot.

## Exit / quit (`commands/quit.rs` + `lib.rs::RunEvent`)
- `WindowEvent::CloseRequested` -> `prevent_close()` + hide (close never exits).
- `RunEvent::ExitRequested` -> if `!state.exit_allowed()`: `api.prevent_exit()` + emit `ui://quit-requested`
  to the frontend; if emitting fails, `allow_exit()` + `exit(0)` so the app can never be stuck.
- `commands::quit::request_quit` shows a native confirm (`OkCancelCustom("退出", "取消")`), then
  `allow_exit()` + `exit(0)`. `begin_quit_prompt`/`quit_pending` prevent two dialogs (tray + shortcut race).
- Frontend must `flush()` autosave **before** calling `requestQuit`; a save error keeps the app running.

## Menu bar / autostart
- Tray icon comes from `src-tauri/icons/tray-logo.png` via `include_bytes!` + `icon_as_template(false)`
  (brand logo, no template tinting); AppKit pins the icon to 18pt height.
- Tray has a `CheckMenuItem` for autostart whose state is **re-read from the system**
  (`app.autolaunch().is_enabled()`) after every toggle; the item handle must be cloned into the closure
  because `AppHandle` has no `app.menu()`.
- Tray events are menu ids: `toggle`, `new_note`, `import`, `library`, `settings`, `autostart`, `quit`.
  `import` calls `window.show` (not the `toggle` helper): the modal needs a visible window.

## Long-running work (`state.rs`)
- One import at a time: `begin_import()` swaps an `AtomicBool` and returns an `ImportGuard` whose `Drop`
  clears both `running` and `cancel`; `request_import_cancel()` is a no-op when nothing runs, and a stray
  cancel flag is reset by the next `begin_import()`. Logic itself lives in `services/`, so integration
  tests drive it without a Tauri runtime; the command layer only emits `ui://import-progress`.

## Migrations (`database/migrations.rs`)
- Append-only `MIGRATIONS: &[(i64, &str)]` with `include_str!("../../migrations/NNNN_*.sql")`;
  one transaction per migration + `schema_migrations` row; `LATEST_VERSION` = last entry.
- `ensure_not_newer` refuses to open a library written by a newer build.
- A migration that changes the FTS projection format must be listed in `REINDEX_VERSIONS` so
  `search_index::reindex_all` runs right after it (CJK explosion cannot be expressed in SQL).
