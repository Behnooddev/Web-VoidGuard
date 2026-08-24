# VoidGuard

A local Windows security and system-monitoring desktop application:
process/service/network/port visibility, firewall and DNS management,
file-integrity and startup monitoring, event correlation, risk
scoring, and a full audit trail.

VoidGuard is a **defensive** tool for systems you own or are authorized
to administer. It has no remote-control, shell-execution, or
persistence capability of its own — see `SECURITY.md`.

## Repository contents

- `src/`, `src-tauri/` — the application itself (see above)
- `handoffs/` — one document per completed/in-progress phase: what
  shipped, what's left, and what needs verifying next
- `wiki/` — source markdown for the GitHub Wiki (copy into the Wiki
  tab, or clone `<repo>.wiki.git` and drop these in)
- `docs/` — the informational GitHub Pages site (`OWNER.github.io/voidguard/`).
  **Documentation only** — it has no connection to and cannot control
  any device, including the one running VoidGuard
- `.github/` — issue templates, PR template, CI workflow, Pages deploy workflow

> Replace `OWNER` in `docs/index.html`, `wiki/*.md`, and this README
> with the actual GitHub username/org once the repo is created.

## Status

This repository implements **Phases 1 through 5** of the plan in
`ARCHITECTURE.md`: app shell, dashboard, process manager, open port
monitoring/control, network adapters, services manager, file
integrity monitoring, startup/persistence monitoring, a filterable
event timeline, a starter risk-correlation engine, full firewall rule
management, per-interface DNS, a real scanning system, an explained
security score, and configurable desktop notifications are all
functional. Scheduled Tasks (one of several startup-persistence
vectors) and the Audit Log page aren't covered yet. See `handoffs/`
for the detailed, dated record of what shipped in each phase —
including a real Windows compile pass in Phase 4 that found and fixed
an actual bug (see `handoffs/04-phase-4-handoff.md`), and debugging
checklists for what's still unverified in later phases. Every
remaining sidebar page is an explicit, labeled "not implemented yet"
state — nothing in the UI is faked.

## Stack

- **Frontend:** React + TypeScript + Tailwind CSS + Lucide icons + Recharts
- **Shell:** Tauri (Rust backend, WebView2 frontend)
- **Backend:** Rust (`sysinfo`, `rusqlite`, `windows` crate for native APIs)
- **Database:** SQLite (bundled, WAL mode)

## Prerequisites (Windows 10/11)

- [Rust](https://rustup.rs) (stable toolchain)
- [Node.js](https://nodejs.org) 18+
- [Tauri prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites) — Microsoft Visual Studio C++ Build Tools, WebView2 (preinstalled on Win11)

## Setup

```powershell
npm install
npm run tauri dev
```

## Build a release installer

```powershell
npm run tauri build
```

Produces an MSI and NSIS installer under `src-tauri/target/release/bundle/`.

## Project layout

```
src/                  React frontend
  components/         Sidebar, cards, shared UI
  pages/               One page per sidebar section
  lib/                 IPC wrapper (src/lib/ipc.ts) + utils
  types/               TypeScript types mirroring Rust models
src-tauri/
  src/main.rs          App entry, command registration, metrics emitter
  src/commands/        One #[tauri::command] module per subsystem
  src/db/              SQLite init + migrations
  src/models/          Shared Rust structs (events, audit, metrics)
```

See `ARCHITECTURE.md` for the module/data-flow breakdown and the
phase-by-phase build plan, `SECURITY.md` for the privilege boundary,
`DEVELOPMENT.md` for day-to-day dev workflow and testing, and
`RELEASING.md` for how tagged releases are auto-drafted and published.
