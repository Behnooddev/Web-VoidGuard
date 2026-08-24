# Architecture

## Layers

```
React UI (src/)
   |  invoke() / listen()  — typed requests only, never raw strings
Tauri IPC
   |
Application Core (src-tauri/src/commands/*)
   |
Security / Monitoring Services (per-subsystem logic, risk/event engines)
   |
Windows System Adapters (sysinfo, `windows` crate, WinAPI wrappers)
   |
Windows APIs
```

The frontend **never** performs a privileged operation directly. Every
mutation (DNS change, firewall rule edit, service start/stop, process
kill) is a typed Rust command that validates its input, calls the
appropriate Windows adapter, and writes an audit entry — success or
failure — before returning a structured result to the UI.

## Modules

| Module | Responsibility | Status |
|---|---|---|
| `commands::system` | CPU/RAM/disk/uptime metrics via `sysinfo` | Phase 1 done |
| `commands::events` | Insert/query normalized `events` rows | Phase 1 done |
| `commands::audit` | Append-only `audit_logs` writes/reads | Phase 1 done |
| `commands::process` | Process listing, details, safe termination | Phase 2 — done |
| `commands::network` | Adapter enumeration (IPv4/IPv6/gateway/DNS/MAC) | Phase 2 — done |
| `commands::ports` | Listening endpoints, terminate owner, open/close port via Firewall COM API | Phase 2 — done (Windows-native, unverified compile — see `handoffs/02-phase-2-handoff.md`) |
| `commands::services` | SCM enumeration + start/stop/restart/startup-type | Phase 2 — done (Windows-native, unverified compile) |
| `commands::files` | File-integrity watcher (`notify` crate) over configured scopes | Phase 3 — done (Windows-native, unverified compile) |
| `commands::startup` | Run/RunOnce keys, Startup folders; scheduled tasks not yet covered | Phase 3 — mostly done (Windows-native, unverified compile) |
| `commands::risk` | Event correlation → composite risk score | Phase 3 — done, 2 starter rules (Windows-native, unverified compile) |
| `commands::firewall` | Full rule management (beyond single-port open/close, already done in Phase 2) via WFP/`INetFwPolicy2` COM | Phase 4 — done (BSTR/VARIANT_BOOL fix applied — see `handoffs/04-phase-4-handoff.md`) |
| `commands::dns` | Per-interface DNS read/validate/apply | Phase 4 — done (Windows-native, unverified compile) |
| `commands::scan` | Quick/System/Network/Startup/Integrity/Custom scans | Phase 5 — done, real progress + findings (Windows-native, unverified compile — see `handoffs/05-phase-5-handoff.md`) |
| `commands::security_score` | Aggregate scoring engine with explained reasons | Phase 5 — done, 4 signal sources |
| `commands::notifications` | OS notification dispatch, user-configurable | Phase 5 — done (preferences in Rust, OS dispatch client-side) |

Each module owns its own request/response types in `src-tauri/src/models`
and its own SQLite table(s) (see `db::init` migrations) — no shared
mutable state beyond the `Db` and `SysHandle` managed by Tauri.

## Data flow example — DNS change (Phase 4 target shape)

```
UI: user submits ChangeDnsRequest { interface_id, primary_dns, secondary_dns }
  -> validate IPv4/IPv6 syntax client-side (fast feedback)
  -> invoke("change_dns", request)
Backend:
  -> re-validate request (never trust the client)
  -> read current config (adapter API) for the "before" audit value
  -> confirm dialog already acknowledged by user (UI-level gate)
  -> call Windows API to apply new DNS
  -> on success: record_event(DnsChanged, ...), record_audit(SUCCESS, before, after)
  -> on failure: return AppError { code: "DNS_CHANGE_FAILED", ... }, record_audit(FAILURE)
UI:
  -> show structured error or success toast
  -> dashboard/event feed updates via the "system-metrics" / event stream
```

## Real-time updates

A single Tokio interval task (spawned once in `main.rs::setup`) emits a
`system-metrics` event every 2 seconds. This is **not** a busy-poll
loop — it's a fixed-interval timer reusing one long-lived `sysinfo::System`
handle, refreshed in place. Later phases add their own event sources
(filesystem watcher callbacks, WMI/ETW subscriptions for process and
service changes) that push into the same `events` table and emit
their own typed events rather than being polled from the frontend.

## Database

SQLite, WAL mode, one file under `%APPDATA%/VoidGuard/voidguard.db`.
Migrations are forward-only and additive (see `db::run_migrations`);
each phase adds tables, never rewrites earlier ones. Retention/cleanup
of high-volume tables (`events`, `process_snapshots`, `port_snapshots`)
is configurable from Settings and runs automatically on every launch
(`commands::retention`) — see `handoffs/06-phase-6-handoff.md`.

## Security boundary (summary — see SECURITY.md)

- No arbitrary shell/PowerShell/cmd execution, ever.
- No user-controlled strings passed to `Command::new` / equivalent.
- Every privileged action is a named, typed Tauri command.
- Every privileged action writes to `audit_logs`, success or failure.
- Destructive actions (process kill, service stop, firewall rule
  delete, protected-service changes) require explicit UI confirmation
  before the backend command is even invoked.

## Phased build plan

1. Tauri setup, React shell, sidebar/dashboard, SQLite, event+audit backend — **done**
2. Process monitoring, network interfaces, open ports, service monitoring — **done** (native Windows code not yet compiled/tested — see `handoffs/02-phase-2-handoff.md`)
3. File integrity monitoring, startup/persistence monitoring, event engine, risk engine — **done** (Scheduled Tasks now covered — see `handoffs/06-phase-6-handoff.md`; native Windows code not yet fully compiled/tested)
4. Firewall management, DNS management, privileged-operation plumbing, audit UI — **done** (COM string-type bug found and fixed — see `handoffs/04-phase-4-handoff.md`)
5. Scanning system, security scoring, notifications, dashboard polish — **done** — see `handoffs/05-phase-5-handoff.md`
6. Testing, performance/retention, hardening, docs, Windows packaging — **mostly done** — see `handoffs/06-phase-6-handoff.md`
