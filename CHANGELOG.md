# Changelog

All notable changes to this project are documented here.
Format loosely follows [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased] — Phase 6 not started

### Added (Phase 5)
- Scanning system: Quick/System/Network/Startup/Integrity/Custom scans,
  each running real enumeration steps with live progress and persisted
  results/findings. New Scans page.
- Security score: 4-signal explained scoring (firewall status, startup
  classifications, open port risk, recent risk findings), recomputed
  automatically every 10 minutes. New Security page; Dashboard's score
  card is now real.
- Configurable desktop notifications (on/off + minimum severity),
  dispatched client-side via Tauri's Notification API against events
  from the existing event feed. New Settings page (notifications +
  theme).
- New Health page: fuller CPU/RAM/per-disk/uptime view, explicit about
  not showing battery/temperature since Windows can't reliably provide
  those on all hardware.
- See `handoffs/05-phase-5-handoff.md` for the full writeup, known
  gaps, and the Windows-pass checklist.

### Fixed
- **The exact `HSTRING`/`BSTR`/`IntoParam` compiler errors reported
  from a real `npm run tauri dev` run** — `INetFwRule`/`INetFwPolicy2`
  are COM Automation interfaces (`BSTR` strings, `VARIANT_BOOL`
  booleans), not WinRT (`HSTRING`). Fixed in both `firewall.rs` and
  `ports.rs`. See `handoffs/04-phase-4-handoff.md`'s "BSTR vs HSTRING
  fix" section for the full explanation — useful reading before
  writing any more COM Automation code in this project.

## [Unreleased] — Phase 4

### Added
- Full Windows Firewall rule management: create/enable-disable/delete
  named rules (not just single-port allow/block) via `INetFwPolicy2`/
  `INetFwRule` COM, tracked locally so the UI doesn't have to enumerate
  the entire system rule set. New Firewall page.
- Per-interface DNS configuration (static servers or DHCP) via the
  native `SetInterfaceDnsSettings` API, exposed from the Network →
  Adapters tab.
- See `handoffs/04-phase-4-handoff.md` for the full writeup and the
  Windows-pass checklist.

### Fixed
- **Rust build failures reported against `windows 0.54.0`** in
  `services.rs` and `network.rs` (verified against the actual crate
  source, not guessed):
  - `SC_HANDLE` lives in `windows::Win32::Security`, not
    `windows::Win32::System::Services` — fixed the import/usages in
    `services.rs`.
  - Several `windows`-crate flag types (`ENUM_SERVICE_STATE`,
    `GET_ADAPTERS_ADDRESSES_FLAGS`) don't derive `BitOr`, so
    `SERVICE_ACTIVE | SERVICE_INACTIVE` and
    `GAA_FLAG_INCLUDE_PREFIX | GAA_FLAG_INCLUDE_GATEWAYS` didn't
    compile. Replaced with the crate's own `SERVICE_STATE_ALL`
    constant and a manually-combined, re-wrapped
    `GET_ADAPTERS_ADDRESSES_FLAGS` respectively.
  - `ENUM_SERVICE_STATUS_PROCESSW::ServiceStatusProcess.dwCurrentState`
    and `QUERY_SERVICE_CONFIGW::dwStartType` are typed
    (`SERVICE_STATUS_CURRENT_STATE` / `SERVICE_START_TYPE`), not raw
    `u32` — `map_status` and the startup-type match in `services.rs`
    were comparing against the wrong type via stray `.0` access.
  - `ChangeServiceConfigW`'s "leave unchanged" sentinel
    (`SERVICE_NO_CHANGE`) is a bare `u32` in this crate, but the
    parameters that take it are typed `ENUM_SERVICE_TYPE`/
    `SERVICE_ERROR` — now wrapped instead of passed raw.
  - The service-restart `match (stop, start)` in
    `control_service`/`services.rs` was missing the
    `(Ok(_), Err(_))` case (stop succeeded, restart's start failed) —
    added, with its own error message distinct from "stop failed".
  - `IP_ADAPTER_ADDRESSES_LH` has no top-level `Flags` field in this
    crate's generated bindings — it's nested in the `Anonymous2`
    union (`adapter.Anonymous2.Flags`) — fixed in `network.rs`.
  - Removed a leftover unused `PWSTR` import in `services.rs`.
  - `ports.rs` was reviewed line-by-line against the same crate
    source and had no actual bugs — the errors reported against it
    were consistent with cascading diagnostics from the files above,
    not its own issues.
  - **Not an actual bug:** the reported "multiple windows-core
    versions (0.52/0.54/0.61) causing incompatibilities" is expected,
    benign Cargo behavior — `rfd`, `tauri`/`wry`, `generator`,
    `sysinfo`, and `iana-time-zone` (via `chrono`) each pin their own
    internal `windows`/`windows-core` version, and none of them share
    types with our own `windows 0.54` code. It doesn't need fixing;
    see `handoffs/04-phase-4-handoff.md` for the resolved dependency
    breakdown that confirms this.
- `vite.config.ts` had no `resolve.alias` for the `@/*` path used
  everywhere in the frontend, even though `tsconfig.json` defines it —
  `tsc` respects `tsconfig.json` paths on its own, but Vite/Rollup
  don't, so `npm run build` was silently broken since Phase 1. Added
  the matching alias; `vite build` now completes cleanly.
- A pre-existing `tsc` type error in `terminateProcess`
  (`lib/ipc.ts`) from an untyped `invoke()` call.

### Added (Phase 3)
- File integrity monitoring: live `notify`-based watcher over a
  conservative default scope list, SHA-256 hashing, Files page.
- Startup/persistence monitoring: Registry Run/RunOnce + Startup
  folder enumeration with evidence-based classification, removal
  action, Startup page. Scheduled Tasks not yet covered.
- Event Center: full filterable event timeline (Events page).
- Risk engine: two starter correlation rules producing evidence-based
  findings, surfaced on the Events page, run on demand.
- Automated release workflow (`.github/workflows/release.yml`):
  generates categorized release notes from Conventional Commits,
  builds Windows installers, and opens a **draft** GitHub release —
  publishing is always a manual approval step. See `RELEASING.md`.

## Phase 2 completion

### Added
- Network Adapters tab: full adapter list (IPv4/IPv6/gateway/DNS/MAC/
  DHCP/link speed) via native `GetAdaptersAddresses`.
- Services Manager: list, start/stop/restart, and change startup type
  via native Service Control Manager APIs, with an extra-strength
  confirmation for a starter set of protected system services.
- Phase 2 marked complete in `handoffs/02-phase-2-handoff.md`, with a
  prioritized checklist for the first Windows compile/debug pass.

## Phase 2 (process + ports)

### Added
- Process Manager: live process table, search, sort, confirmed termination.
- Open Port Monitoring: TCP/UDP listening endpoints with owning process.
- Port Control: terminate a port's owning process; open/block a port
  via native Windows Firewall COM API (no shell execution).
- Project renamed from the internal working name "WinGuard" to **VoidGuard**.
- Per-phase handoff documents under `handoffs/`.
- GitHub project scaffolding: LICENSE, CONTRIBUTING, CODE_OF_CONDUCT,
  issue/PR templates, CI workflow.
- Project wiki source under `wiki/`.
- Informational GitHub Pages site under `docs/`.

### Known caveats
- Native Windows code (`GetExtendedTcpTable`/`GetExtendedUdpTable`,
  firewall COM interfaces) has not been compiled or run on Windows
  yet — see `handoffs/02-phase-2-handoff.md`.

## [0.1.0] — Phase 1

### Added
- Tauri + React + TypeScript + Tailwind app shell.
- Sidebar navigation (13 sections), dark/light/system theme.
- Live dashboard: CPU, RAM, disk, process count, uptime.
- SQLite schema for every planned subsystem.
- Event log and append-only audit log backend + read APIs.
- Honest "not implemented yet" placeholder for every unbuilt page.
