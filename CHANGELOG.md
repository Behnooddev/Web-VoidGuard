# Changelog

All notable changes to this project are documented here.
Format loosely follows [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased] — Phase 2 (in progress)

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
