# Phase 1 Handoff — App Shell, Dashboard, Event/Audit Backend

**Status:** Complete
**Date:** 2026-08-21

## What shipped

- Tauri + React + TypeScript + Tailwind project scaffold (`package.json`,
  `vite.config.ts`, `tailwind.config.js`, `src-tauri/Cargo.toml`,
  `src-tauri/tauri.conf.json`).
- App shell: collapsible sidebar (13 sections), dark/light/system theme
  with persisted preference, hash-router page switching.
- Dashboard page wired to **live** backend data: CPU, RAM, disk,
  process count, uptime, host name, OS version. Updates automatically
  every 2 seconds via a Tokio interval task emitting a `system-metrics`
  Tauri event (`src-tauri/src/main.rs::setup`) — not polled from the
  frontend.
- SQLite database (`src-tauri/src/db/mod.rs`): WAL mode, foreign keys
  on, full schema for every planned subsystem (events, alerts,
  audit_logs, process_snapshots, network_snapshots, port_snapshots,
  services, firewall_rules, file_events, startup_entries, scan_results,
  settings) created up front so later phases only add rows, not
  migrations that touch existing tables.
- Event backend: `commands::events::record_event` /
  `get_recent_events`, normalized `SystemEvent` model with category,
  severity, source, before/after state.
- Audit backend: `commands::audit::record_audit` / `get_audit_log`,
  append-only, one row per privileged action regardless of outcome.
- Every sidebar page other than Dashboard renders an explicit,
  labeled **"Not implemented yet"** state (`PlaceholderPage.tsx`) that
  names the phase that will build it — nothing in the UI is faked.

## Files of note

| Path | Purpose |
|---|---|
| `src-tauri/src/main.rs` | Command registration, DB/sysinfo state, metrics emitter |
| `src-tauri/src/db/mod.rs` | Schema + migrations |
| `src-tauri/src/models/mod.rs` | Shared Rust structs |
| `src-tauri/src/commands/{events,audit,system}.rs` | Phase 1 commands |
| `src/App.tsx` | Route table, theme handling |
| `src/pages/Dashboard.tsx` | Live metrics + recent events feed |
| `src/lib/ipc.ts` | Typed wrapper around every `invoke()` call |

## Known limitations / not done

- No Windows-specific enrichment anywhere yet (this phase is
  cross-platform via `sysinfo` so it could be iterated on outside
  Windows). Signature/integrity/publisher fields don't exist yet at
  all in Phase 1 — they're introduced as `Option`s in Phase 2's
  process model, explicitly `None` until the Windows adapter fills
  them in.
- No retention/cleanup policy on `events` yet — fine at Phase 1
  volume, must be addressed before Phase 6 hardening.
- Not build-tested on actual Windows yet — development happened
  without a Windows target available, see "Verification still needed"
  below.

## Verification still needed (do this first, on a Windows machine)

1. `npm install && npm run tauri dev` — confirm the app window opens,
   sidebar navigates, dashboard numbers move.
2. Confirm `%APPDATA%/VoidGuard/voidguard.db` is created and has all
   tables (`sqlite3 voidguard.db ".tables"`).
3. Confirm dark/light/system theme toggle persists across restarts.

## Handoff to Phase 2

Phase 2 owner should read `ARCHITECTURE.md`'s module table and follow
the "Adding a new subsystem" recipe in `DEVELOPMENT.md`. The pattern
established here (typed request → validated command → audit +
event write → typed response) is meant to be copied exactly for
network interfaces and services.
