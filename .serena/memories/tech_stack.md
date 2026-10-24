# Tech stack & pins

## Rust (`src-tauri/Cargo.toml`)
- edition 2021, `rust-version = 1.77`; toolchain in use rustc/cargo 1.98 (2026-10).
- Tauri 2 with features `tray-icon`, `image-png`.
- `rusqlite 0.40` with `bundled` (SQLite + FTS5 compiled in; no system sqlite dependency).
- `uuid` v7 for note ids, `thiserror 2` for `AppError`, serde/serde_json for DTOs.
- Plugins (order matters only for single-instance): `single-instance`, `store`, `dialog`, `opener`,
  `clipboard-manager`, `autostart` (`MacosLauncher::LaunchAgent`), `global-shortcut`.
- `tauri-plugin-single-instance` has **no npm counterpart**; single instance is Rust-only.

## Frontend (`package.json`)
- Vue 3.5 (SFC, `<script setup lang=ts>`), Pinia 4, `reka-ui` for headless menus/dialogs.
- TypeScript **5.9.3 pinned on purpose**: `vue-tsc 3.3.13` peer-constrains it; do not bump to 7.x.
- Vite 8 + `@vitejs/plugin-vue` 6; Tailwind 4 via `@tailwindcss/vite` (no `tailwind.config.js`).
- CodeMirror 6 (`@codemirror/{state,view,language,commands,lang-markdown}`); `markdown-it 15` with `html:false`;
  `shiki 4.5` wired through `shiki/core` + JS regex engine (no oniguruma wasm); themes are
  `github-light-high-contrast` and `github-dark`. Language grammars are dynamic imports, so `vite build`
  emits one chunk per language and the main bundle warning is expected.
- Tests: `vitest 5` + `jsdom` + `@vue/test-utils`; config `vitest.config.ts` (alias `@` -> `src`, include `src/**/*.test.ts`).

## Tooling facts
- Package manager **pnpm** (lockfile `pnpm-lock.yaml` is committed).
- TS alias `@/*` -> `src/*` (tsconfig `paths`, mirrored in `vite.config.ts` and `vitest.config.ts`).
- `strict`, `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch` are on -> dead locals break `vue-tsc`.
- **No** ESLint / Prettier / rustfmt / clippy config in the repo: format by hand, match surrounding style.
- OpenSpec CLI 1.11 (installed globally under the nvm node 24 prefix) drives the spec workflow.
