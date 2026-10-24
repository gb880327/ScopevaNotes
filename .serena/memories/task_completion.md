# Definition of done

Run from repo root unless noted:

1. `pnpm vue-tsc --noEmit`
2. `pnpm vitest run`
3. `pnpm vite build`
4. `cd src-tauri && cargo test`
5. `cd src-tauri && cargo check` (only needed when the test build did not already cover it; `cargo test` compiles lib+bin)

Extra gates by change type:
- Schema / `search_index` / migration change: `cargo test --release --test search_bench -- --ignored --nocapture`
  (p95 budget) **and** bump a new migration instead of editing an applied one.
- Any artifact edit: `openspec validate v1-notes-manager --strict`.
- Frontend behaviour change: exercise it in the real app (`pnpm tauri dev`), not just unit tests.
- Rust startup/tray/shortcut/quit change: `pnpm tauri dev` must start without panic; stop it with
  `pkill -f "target/debug/macos-notes"` afterwards.

Rules the spec enforces (do not violate to make a checklist green):
- Never mark a task `[x]` in `openspec/changes/v1-notes-manager/tasks.md` unless it was actually
  verified; macOS UI items that need Screen Recording/Accessibility permissions are left unchecked
  and reported as manual steps.
- Report commands run + their outcome, and name what stayed unverified.

## Where acceptance evidence lives
`openspec/changes/v1-notes-manager/verification.md` maps every spec scenario (7 capabilities, 100
scenarios) to its evidence: automated test name, manual observation, or "pending". When a task is
marked `[x]`, its scenarios must appear there as automated or observed; when a scenario has no test,
either add one or list it as pending — do not let the matrix drift.

## Release / perf commands
- `pnpm tauri build --target universal-apple-darwin` (needs `rustup target add aarch64-apple-darwin`);
  verify with `lipo -info .../Notes.app/Contents/MacOS/macos-notes`.
- `scripts/perf-check.sh startup` reports process / library-open / WebKit milestones.
- `scripts/perf-check.sh soak 24` samples CPU% (diff of `ps -o time=`) and RSS every 20 s.
  macOS `ps -o %cpu` is an average since process start, never use it for idle measurements.
