# Phase 2 Handoff — Process, Ports, Network, Services

**Status:** Complete (pending Windows compile/debug pass — see checklist at the end)
**Date:** 2026-08-22

## What shipped

### Process Manager (complete)
- `commands::process::list_processes` — full process table via `sysinfo`
  (pid, parent pid, name, exe path, CPU%, memory, start time).
- `commands::process::terminate_process` — explicit, confirmed-only kill.
- `src/pages/ProcessesPage.tsx` — search, column sort, confirm-before-kill
  dialog, structured error display on failed termination.
- Windows-specific enrichment (Authenticode signature, publisher,
  integrity level, SHA-256, live network-connection count) is modeled
  as `Option` fields, currently always `None`. This is the next piece
  of Phase 2/3 work, not a Phase 1 gap.

### Open Port Monitoring + Control (complete, Windows-only — see caveat below)
Pulled forward from the original Phase 4 firewall scope at the user's
request, since "see what's on a port / kill it / open or close the
port" was wanted now rather than later:

- `commands::ports::list_listening_ports` — enumerates TCP + UDP
  listening endpoints via `GetExtendedTcpTable` / `GetExtendedUdpTable`
  (native `windows` crate, no shelling out), resolving each PID's
  process name and full image path via `QueryFullProcessImageNameW`.
- `commands::ports::terminate_port_owner` — kills the process bound to
  a port; thin wrapper over `process::terminate_process` that audits
  and logs the action as a **port** action specifically.
- `commands::ports::open_port` / `close_port` — creates/removes a
  single named Windows Firewall rule scoped to exactly one
  port + protocol + direction, via the native `INetFwPolicy2` /
  `INetFwRule` COM interfaces (the same API the Firewall control panel
  itself uses). No `netsh`, no shell invocation anywhere.
- `src/pages/NetworkPage.tsx` — table of listening ports with risk
  badges, and three confirm-gated actions per row: terminate owner,
  block port, allow port. Every action shows a distinct confirmation
  dialog explaining exactly what will happen before the backend is
  called.
- Every port action (terminate/open/close) writes an audit_logs row
  and, on success, an events row (`PORT_CLOSED` / `FIREWALL_CHANGED`).

## ⚠️ Compile/runtime caveat — read before merging

This module was written in a Linux sandbox with **no Windows target
available to compile or run against**. The `windows` crate API
surface used here (`GetExtendedTcpTable`, `GetExtendedUdpTable`,
`INetFwPolicy2`, `INetFwRule`, `QueryFullProcessImageNameW`, buffer
sizing via a null-then-real call pattern) is written to the best of
available knowledge of the real Win32/COM signatures, but:

- Struct layouts (`MIB_TCPTABLE_OWNER_PID`, `MIB_UDPTABLE_OWNER_PID`)
  use a variable-length trailing array (`table: [ROW; ANYSIZE_ARRAY]`)
  — the raw-pointer slice construction in `enumerate_tcp`/`enumerate_udp`
  needs to be verified against whatever layout the installed
  `windows` crate version actually generates.
- COM interface method names/signatures for `INetFwRule` setters
  (`SetLocalPorts`, `SetProfiles`, etc.) should be diffed against the
  `windows` crate version pinned in `Cargo.toml` (`0.54`) — these APIs
  have shifted across crate versions before.
- **First task for whoever picks this up on a real Windows box:**
  `cargo check` in `src-tauri/`, fix whatever the compiler flags, add
  the integration test described in `DEVELOPMENT.md` under
  `#[cfg(windows)]`.

None of this affects the frontend, the audit/event plumbing, or the
non-Windows build path (`#[cfg(not(windows))]` correctly returns
`AppError::not_supported` everywhere).

## Remaining in Phase 2

None — all four subsystems (process, ports, network adapters,
services) are implemented. What's left is verification, not new
scope; see the debugging checklist below.

### Network Adapters (complete)
- `commands::network::list_network_adapters` — enumerates every
  adapter via `GetAdaptersAddresses` (Ethernet/Wi-Fi/VPN/loopback/
  other), parsing friendly name, description, oper status, MAC,
  IPv4/IPv6 unicast addresses, default gateway, DNS servers, DHCP
  flag, and link speed.
- `src/components/AdaptersTab.tsx` — card grid, one card per adapter,
  auto-refreshes every 10s.
- `src/pages/NetworkPage.tsx` now has two tabs: **Adapters** (new) and
  **Open Ports** (from earlier in Phase 2) — one sidebar entry, two
  views, matching the original spec's "Network Center" + "Open Port
  Monitor" sections living under one page.

### Services Manager (complete)
- `commands::services::list_services` — enumerates all services via
  `EnumServicesStatusExW`, then queries each one's startup type and
  binary path via `OpenServiceW` + `QueryServiceConfigW`.
- `commands::services::control_service` — start/stop/restart, audited
  and event-logged regardless of outcome. Restart is implemented as
  stop → fixed 1.5s sleep → start; **should be polling
  `QueryServiceStatusEx` instead of sleeping** — flagged for the
  debugging pass, not correct as shipped.
- `commands::services::change_service_startup_type` — Automatic /
  Manual / Disabled via `ChangeServiceConfigW`. "Automatic (Delayed)"
  is modeled in the `StartupType` enum but currently maps to the same
  `SERVICE_AUTO_START` as plain Automatic — the delayed-start flag
  needs `ChangeServiceConfig2W` with `SERVICE_CONFIG_DELAYED_AUTO_START_INFO`,
  not yet wired up.
- A small hardcoded `PROTECTED_SERVICES` list (TrustedInstaller,
  WinDefend, SamSs, etc.) drives an extra-strength confirmation dialog
  in `ServicesPage.tsx` — intentionally conservative and incomplete,
  meant as a starting point, not a definitive list.
- `src/pages/ServicesPage.tsx` — table with per-row start/stop/restart
  buttons (disabled when already in that state) and an inline startup
  type `<select>`.

## Full debugging checklist for the Windows pass (do this first)

Everything below was written without a Windows compiler available in
this sandbox. In rough priority order:

1. **`cargo check` in `src-tauri/`** and fix whatever the compiler
   flags first — expect issues in the raw-pointer/struct-layout code
   (`ports.rs`'s `MIB_TCPTABLE_OWNER_PID`/`MIB_UDPTABLE_OWNER_PID`
   variable-length array access, `network.rs`'s
   `IP_ADAPTER_ADDRESSES_LH` linked-list walk) before anything else.
2. **COM interfaces** (`ports.rs::windows_impl::set_port_rule`) — diff
   `INetFwPolicy2`/`INetFwRule` method names against the `windows`
   crate `0.54` docs; these have moved across crate versions before.
3. **Service Control Manager calls** (`services.rs`) — verify
   `EnumServicesStatusExW` buffer-sizing pattern, `QueryServiceConfigW`
   struct parsing, and that `ChangeServiceConfigW`'s many `None`
   parameters match the crate's expected signature.
4. **Fix the restart sleep** in `services.rs::control_service` — poll
   status instead of a fixed delay.
5. **Wire up delayed-auto-start** via `ChangeServiceConfig2W` if you
   want `AUTOMATIC_DELAYED` to actually do something different from
   `AUTOMATIC`.
6. Once it compiles: run the app, open **Processes**, **Network**
   (both tabs), and **Services**, and confirm real data appears and
   every action (terminate, open/close port, start/stop/restart
   service, change startup type) works and produces the expected
   `audit_logs` + `events` rows.
7. Add the `#[cfg(windows)]` integration tests mentioned in
   `DEVELOPMENT.md` once the above is confirmed working, so the next
   phase doesn't regress this one.

None of the above affects the frontend, the DB schema, or the
non-Windows fallback paths — those are straightforward TypeScript/SQL
and don't need a Windows machine to review.

## Handoff to Phase 3

Phase 3 owner should read `ARCHITECTURE.md`'s module table and the
"Adding a new subsystem" recipe in `DEVELOPMENT.md`. File integrity
monitoring, startup/persistence monitoring, and the event correlation
/ risk engine are next. The `notify` crate dependency for filesystem
watching is already in `Cargo.toml` from Phase 1, unused until now.

