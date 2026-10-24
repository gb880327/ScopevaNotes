# Commands

## Run
- `pnpm install` — first run; lockfile committed.
- `pnpm tauri dev` — full app (Rust + Vite on :1420). Leaves a `target/debug/macos-notes`
  process running; stop it with `pkill -f "target/debug/macos-notes"` (also kills the tauri CLI child after a few seconds).
- `pnpm dev` — Vite only. Not a usable app: `services/commands.ts` throws a Chinese
  'run inside Tauri' error when `isTauri()` is false.
- `pnpm tauri build` — unsigned `.app` + `.dmg`.

## Verify
- `pnpm vue-tsc --noEmit` — typecheck (also part of `pnpm build`).
- `pnpm vitest run` — frontend unit tests.
- `pnpm vite build` — production bundle only.
- `cd src-tauri && cargo test` — unit + integration tests (fast, ~2s).
- `cd src-tauri && cargo check` — cheaper than a full test run during iteration.

## Benchmark (opt-in, release build takes minutes, then ~4s)
- `cd src-tauri && cargo test --release --test search_bench -- --ignored --nocapture`
  Prints seed time, library size, p50/p95; asserts p95 < 100 ms at 10k notes.
  The test is `#[ignore]`d so plain `cargo test` stays fast.

## OpenSpec (acceptance baseline, see `mem:specs_workflow`)
- `openspec status --change v1-notes-manager` / `--json` — artifact + task progress numbers.
- `openspec instructions apply --change v1-notes-manager --json` — next unchecked tasks.
- `openspec validate v1-notes-manager --strict` — must pass after any artifact edit.
- `openspec list`, `openspec archive ...` for lifecycle.

## Darwin specifics
- `timeout` is **not installed**: to run a long-lived process in a script use `(cmd > /tmp/x.log 2>&1 &)` + `sleep N` + `tail`.
- `gtimeout` also absent; prefer bounded `sleep` loops over waiting on a process.
- Sandboxed-terminal `screencapture` returns black frames and `osascript` UI scripting fails with
  `-25211` unless Terminal has Screen Recording / Accessibility grants -> UI must be verified by the user.
