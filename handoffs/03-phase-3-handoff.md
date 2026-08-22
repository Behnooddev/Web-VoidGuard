# Phase 3 Handoff — File Integrity, Startup, Event Engine, Risk Engine

**Status:** Mostly complete (scheduled tasks not covered; pending
Windows compile/debug pass — see checklist at the end, same as Phase 2)
**Date:** 2026-08-22

## What shipped

### File integrity monitoring
- `commands::files::init_and_start_watching` — seeds a small,
  conservative default watch list (hosts file, per-user and
  all-users Startup folders) into the new `watch_scopes` table on
  first run, then starts a live `notify` watcher over every scope
  currently in that table. Started once from `main.rs::setup`, not
  polled.
- Filesystem events are hashed (SHA-256, `sha2` crate) when the file
  still exists, written to `file_events`, and mirrored into the
  shared `events` table (`FILE_CREATED`/`FILE_MODIFIED`/`FILE_DELETED`,
  severity Low) so they show up in the Events timeline too.
- The watcher runs on a plain OS thread (not Tokio) with its own
  SQLite connection, since `notify`'s channel is synchronous.
- `src/pages/FilesPage.tsx` — watched-location list + recent file
  event feed, explicit note that it's not scanning the whole disk.
- **Not done:** user-configurable watch scopes (add/remove from the
  UI) — the table and read command exist, but there's no `add_watch_scope`/
  `remove_watch_scope` command yet, and the watcher doesn't currently
  support adding a scope without restarting the app.

### Startup / persistence monitoring
- `commands::startup::list_startup_entries` — reads
  `HKLM`/`HKCU\...\Run` and `RunOnce`, plus both Startup folders.
  Each entry gets an evidence-based classification (Known/Unknown/
  Suspicious) — see `windows_impl::classify` — flagging script
  interpreters, TEMP/Downloads paths, and obfuscated PowerShell
  invocations as Suspicious; everything else is Unknown (signature
  checking isn't implemented, so nothing is auto-marked Known yet).
- `commands::startup::remove_startup_entry` — deletes a registry
  value or startup-folder file, confirmed in the UI first, audited
  either way.
- `src/pages/StartupPage.tsx` — table with a hover tooltip showing
  the evidence behind each classification, and a remove action.
- **Not done:** Scheduled Tasks. This needs the Task Scheduler COM API
  (`ITaskService`/`ITaskFolder`), which is a different shape of code
  from the registry/folder approach and was left out of this pass
  rather than stubbed with fake data. The UI says so explicitly.
- **Known correctness issue:** `persist_and_diff` in `startup.rs` uses
  an awkward recursive re-entry to work around a borrow-checker issue
  (dropping the DB lock mid-loop before calling `record_event`). It
  works but is O(n²) and hard to follow — **should be rewritten as a
  plain loop that queues events into a `Vec` and records them after
  the loop ends**, not during the Windows debugging pass necessarily,
  but before this ships.

### Event engine
- `src/pages/EventsPage.tsx` replaces the placeholder: full event
  timeline (100 most recent) with category filter chips (All /
  Security / Network / Files / Processes / Services), reusing the
  Phase 1 `get_recent_events` backend — no new Rust needed here, this
  was a frontend-only gap.

### Risk engine
- `commands::risk::run_risk_analysis` — two starter correlation
  rules, each producing a `RiskFinding` with severity, confidence,
  evidence list, and a plain-language remediation suggestion (never a
  bare score):
  1. **Suspicious/Unknown startup entry + recent port open** (24h
     window) — Medium confidence.
  2. **≥3 new startup entries within an hour** — Low confidence,
     independent of individual classification.
- Findings persist to the new `risk_findings` table and surface at
  the top of the Events page. Analysis runs on-demand via a "Run risk
  analysis" button — **not yet scheduled automatically**; that's a
  reasonable Phase 5 (scoring/dashboard polish) follow-up once there's
  a natural place to trigger it (e.g. after every scan).

## Full debugging checklist for the Windows pass

Same situation as Phase 2 — written without a Windows compiler
available. Everything from `handoffs/02-phase-2-handoff.md`'s
checklist still applies, plus:

1. **Registry code** (`startup.rs::windows_impl`) — verify
   `RegEnumValueW`'s buffer/type-out-param signature against the
   pinned `windows` crate version; the fixed-size `[u16; 256]` /
   `[u16; 2048]` buffers are a simplification that will truncate
   unusually long value names/data — fine for a first pass, worth
   flagging if it comes up in testing.
2. **`notify` crate on Windows** — confirm `RecommendedWatcher` (which
   uses `ReadDirectoryChangesW` under the hood on Windows) behaves as
   expected for the configured scopes, especially the non-recursive
   `hosts` folder watch.
3. **Fix `persist_and_diff`'s recursion** (see above) before relying
   on it for anything beyond manual testing.
4. Once compiling: trigger a file change in a watched folder and
   confirm it shows up in **Files** and **Events**; add/remove a
   registry Run entry and confirm **Startup** picks it up and Remove
   works; click "Run risk analysis" on **Events** and confirm findings
   appear when the trigger conditions are manufactured (e.g. add an
   obviously-suspicious startup entry, then open a port).

## Handoff to Phase 4

Phase 4 owner: full firewall rule management (beyond the single-port
open/close already shipped in Phase 2) and DNS management. Follow the
same `#[cfg(windows)]` adapter pattern. The COM firewall plumbing in
`ports.rs::windows_impl::set_port_rule` is a reasonable starting point
to generalize into full rule CRUD.
