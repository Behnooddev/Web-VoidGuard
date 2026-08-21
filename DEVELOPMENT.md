# Development Guide

## Requirements

- Rust stable (`rustup default stable`)
- Node.js 18+
- Windows 10/11 for full native-API testing (some Phase 1 pieces —
  UI shell, dashboard, event/audit backend — build and run on
  macOS/Linux for frontend iteration, but all `windows`-crate code is
  Windows-only by `#[cfg(windows)]` and won't compile elsewhere)

## Day to day

```powershell
npm install
npm run tauri dev      # hot-reloading UI + auto-rebuilding Rust backend
```

Rust-only iteration (no UI):

```powershell
cd src-tauri
cargo check
cargo test
```

## Adding a new subsystem (the repeatable pattern)

Every module from Phase 2 onward follows the same five steps —
`commands/events.rs` and `commands/audit.rs` are the reference
implementations:

1. Add request/response structs to `src-tauri/src/models/mod.rs`.
2. Add the SQLite table(s) to `db::run_migrations` (additive only —
   never edit a migration that has already shipped).
3. Implement the adapter logic + a `#[tauri::command]` handler in a
   new `src-tauri/src/commands/<name>.rs`.
4. Register the command in `main.rs`'s `invoke_handler!` and add
   `mod <name>;` to `commands/mod.rs`.
5. Add the typed wrapper to `src/lib/ipc.ts` and build the page in
   `src/pages/`, replacing that route's `PlaceholderPage` in `App.tsx`.

## Testing

- `cargo test` in `src-tauri/` for: risk scoring, event normalization,
  DNS validation, firewall request validation, repository functions,
  file-event processing, process/network parsing, config handling.
- Integration tests for Windows adapters live under
  `src-tauri/tests/` and are gated behind `#[cfg(windows)]` — they
  only run in CI on a Windows runner.
- Frontend: component tests are not yet wired up; add
  Vitest + React Testing Library when the first interactive
  (non-read-only) page ships (Phase 2's process manager).

## Database inspection

The SQLite file lives at `%APPDATA%/WinGuard/winguard.db`. Any SQLite
browser (e.g. DB Browser for SQLite) can open it directly — do this
rather than adding ad-hoc debug `println!`s for data you can just
query.

## Style / conventions

- Rust: `cargo fmt`, `cargo clippy -- -D warnings` before committing.
- TypeScript: functional components, no class components; shared
  types live in `src/types`, mirrored 1:1 from the Rust `models`
  structs — keep them in sync by hand (no codegen yet).
- Every new privileged command must call `commands::audit::record_audit`
  on both its success and failure paths — see SECURITY.md.
