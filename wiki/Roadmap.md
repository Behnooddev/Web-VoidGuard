# Roadmap

VoidGuard is built in phases, each with a handoff document in the
repo's `handoffs/` folder once complete.

| Phase | Scope | Status |
|---|---|---|
| 1 | Tauri setup, app shell, dashboard, SQLite schema, event + audit backend | ✅ Done — [handoff](https://github.com/OWNER/voidguard/blob/main/handoffs/01-phase-1-handoff.md) |
| 2 | Process monitoring, open ports (+ terminate/open/close), network interfaces, service monitoring | ✅ Done — [handoff](https://github.com/OWNER/voidguard/blob/main/handoffs/02-phase-2-handoff.md) (native Windows code not yet compiled/tested) |
| 3 | File integrity monitoring, startup/persistence monitoring, event engine, risk engine | ✅ Mostly done — [handoff](https://github.com/OWNER/voidguard/blob/main/handoffs/03-phase-3-handoff.md) (Scheduled Tasks not covered; native Windows code not yet compiled/tested) |
| 4 | Full firewall rule management, DNS management, remaining privileged-operation plumbing, audit log UI | ⏳ Not started (port-level firewall control already shipped early, in Phase 2) |
| 5 | Scanning system, security scoring, notifications, dashboard polish | ⏳ Not started |
| 6 | Testing, performance/retention, hardening, docs, Windows packaging | ⏳ Not started |

For the detailed module-by-module breakdown, see [[Architecture]].
