# Architecture

This is a condensed version of the repo's `ARCHITECTURE.md` — see
that file for the full module table and data-flow example.

## Layers

```
React UI  →  Tauri IPC (typed commands only)  →  Rust command handlers
   →  Security / Monitoring services  →  Windows adapters  →  Windows APIs
```

The frontend never performs a privileged operation directly. Every
mutation goes through a typed Rust command that validates input,
calls the Windows API, and writes an audit entry before returning.

## Current module status

| Module | Status |
|---|---|
| System metrics (CPU/RAM/disk/uptime) | ✅ Done |
| Event log | ✅ Done |
| Audit log | ✅ Done |
| Process manager | ✅ Done |
| Open port monitoring + terminate/open/close | ✅ Done (Windows-native, unverified compile — see [[Port-Control]]) |
| Network adapter enumeration | ✅ Done (Windows-native, unverified compile) |
| Services manager | ✅ Done (Windows-native, unverified compile) |
| File integrity monitoring | ⏳ Not started |
| Startup/persistence monitoring | ⏳ Not started |
| Event correlation / risk engine | ⏳ Not started |
| Full firewall rule management | ⏳ Not started (port-level open/close only so far) |
| DNS management | ⏳ Not started |
| Scanning system | ⏳ Not started |
| Security scoring | ⏳ Not started |
| Notifications | ⏳ Not started |

## Real-time updates

A single Tokio interval task emits a `system-metrics` event every 2
seconds, reusing one long-lived `sysinfo::System` handle refreshed in
place — not a busy-poll loop and not repeated construction of a new
handle. Later subsystems (filesystem watcher, service/process change
detection) will push their own typed events into the same pipeline.

## Database

SQLite, WAL mode, one file at `%APPDATA%/VoidGuard/voidguard.db`.
Every planned subsystem already has its table from Phase 1 — later
phases add rows, not migrations that rewrite earlier tables.

See [[Security-Model]] for the privilege boundary in detail.
