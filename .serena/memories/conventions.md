# Conventions

## IPC contract (frontend <-> Rust)
- `src/services/commands.ts` is the **only** file that may call `invoke`; everything else imports from it.
- Every command returns `Result<T, AppError>`; `AppError` serialises to `{kind, message}` and
  `CommandError` on the TS side preserves `kind` (`not_found` / `conflict` / `invalid` / `database` / ...).
  Branch on `kind`, never on message text.
- DTOs are `#[serde(rename_all = camelCase)]` in `src-tauri/src/models/`; the mirrored TS types live in
  `src/services/types.ts`. Change both in the same commit, Rust first.
- Adding a command requires: module fn in `src-tauri/src/commands/<area>.rs`, `pub mod` already in
  `commands/mod.rs`, registration in the `invoke_handler(tauri::generate_handler![...])` list in `lib.rs`,
  wrapper in `commands.ts`, type in `types.ts`.
- Rust -> frontend messages are Tauri events named `ui://...` (`ui://new-note`, `ui://open-settings`,
  `ui://open-library`, `ui://quit-requested`).

## Rust layering
- `commands/` = thin async adapters only; all SQLite work goes through `commands::blocking` (`spawn_blocking`).
- Business rules live in `repositories/*`; there is **no** general service layer (decision D1),
  only `services/library_service.rs` for path/config concerns.
- `Database` owns one `Mutex<Connection>`; access only via `Database::with_conn(|conn| ...)`.
  Never hold the connection across an `await`; `AppState::db()` hands out an `Arc<Database>` snapshot.
- Errors: return `AppResult<T>`; use `AppError::NotFound/Conflict/Invalid` for domain failures, no panics.
- `///` doc comments are **not allowed on function parameters** (Rust compiler error); use `//` there.

## Naming / files
- Rust files `snake_case`; TS components `PascalCase.vue`, composables `use*.ts`, stores `src/stores/<domain>.ts`.
- Frontend layout: `components/layout/` (structure) vs `features/<domain>/` (behaviour), `windows/MainWindow.vue`
  is the shell that wires stores to layout.
- User-facing strings are Chinese; comments/identifiers are English. Match the surrounding language.

## Spec discipline
- Behaviour changes require a spec/design/tasks edit in `openspec/changes/v1-notes-manager/` in the same
  piece of work; tick `tasks.md` only for work actually verified (see `mem:task_completion`).
