# Phase 2 Handoff — Process, Ports, Network, Services

**Status:** In progress — Process Manager and Open Port Control shipped;
Network Adapters and Services Manager remaining.
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

- `commands::network` — adapter enumeration (Wi-Fi/Ethernet/VPN,
  IPv4/IPv6/gateway/DNS/DHCP/MAC/link speed). Section 9 of the
  original spec; not started.
- `commands::services` — SCM enumeration + start/stop/restart/startup
  type change, with protected-service confirmation. Not started.
- `network_snapshots` and `services` tables already exist in the DB
  schema (Phase 1) and are unused until this lands.

## Handoff to whoever continues Phase 2

Follow the exact same shape as `ports.rs`: cross-platform command
function → `#[cfg(windows)] mod windows_impl` doing the real work →
`#[cfg(not(windows))]` fallback returning `AppError::not_supported`.
Reuse `record_audit` / `record_event` from `commands::audit` /
`commands::events` for every mutating action.
