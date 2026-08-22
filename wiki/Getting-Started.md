# Getting Started

## Requirements

- Windows 10 or 11
- [Rust](https://rustup.rs) (stable toolchain)
- [Node.js](https://nodejs.org) 18+
- [Tauri prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites) — MSVC Build Tools, WebView2 (preinstalled on Win11)

## Clone and run in dev mode

```powershell
git clone https://github.com/OWNER/voidguard.git
cd voidguard
npm install
npm run tauri dev
```

This opens the app window with hot-reloading on the frontend and
auto-rebuild on the Rust backend.

## Build a release installer

```powershell
npm run tauri build
```

Produces an MSI and NSIS installer under
`src-tauri/target/release/bundle/`.

## First run

- The dashboard should immediately show live CPU/RAM/disk/process
  numbers, updating every couple of seconds.
- **Processes**, **Network** (Adapters + Open Ports tabs),
  **Services**, **Files**, **Startup**, and **Events** are fully
  functional. Everything else in the sidebar may show a "not
  implemented yet" card — see the [[Roadmap]] for what's built and
  what isn't yet.
- The database is created at `%APPDATA%/VoidGuard/voidguard.db`.

## Where things live

| What | Where |
|---|---|
| Frontend | `src/` |
| Rust backend | `src-tauri/src/` |
| Per-subsystem commands | `src-tauri/src/commands/*.rs` |
| Database schema | `src-tauri/src/db/mod.rs` |
| Docs | `ARCHITECTURE.md`, `SECURITY.md`, `DEVELOPMENT.md` |
| Phase handoffs | `handoffs/` |

See [[Architecture]] for the full module map and
`CONTRIBUTING.md` in the repo root before sending a PR.
