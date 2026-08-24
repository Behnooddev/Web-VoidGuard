# Phase 6 Handoff — Scheduled Tasks, Audit UI, Retention, Hardening Pass

**Status:** Core scope done; packaging/CI and a real Windows test pass
are still open — see checklist below.
**Date:** 2026-08-23

## What shipped

### Scheduled Tasks monitoring (`commands::startup`)
Closes the gap flagged since Phase 3: startup persistence scanning now
covers Task Scheduler, not just the registry Run keys and the Startup
folder.

- Walks Task Scheduler recursively from the root folder (`\`) via
  `ITaskService`/`ITaskFolder`, including hidden tasks
  (`TASK_ENUM_HIDDEN`) — malware persistence sometimes hides a few
  folders down, not just at the root.
- Pulls each task's actual command out of its XML definition
  (`IRegisteredTask::Xml`) with a small tag-extraction helper rather
  than a full XML parser — task XML is simple and predictable enough
  that this is reliable without adding a dependency.
- Feeds into the same `classify()` heuristic already used for
  registry/startup-folder entries, so a scheduled task pointing at
  something in `%TEMP%` gets flagged the same way a Run-key entry
  would.
- Removal works too: `ITaskFolder::DeleteTask`. Since tasks aren't
  stored with their folder path in the DB (only name + command),
  removal re-walks Task Scheduler once to find the matching task and
  its parent folder before deleting — see the comment on
  `remove_scheduled_task` in `startup.rs` for why.
- New Cargo feature: `Win32_System_TaskScheduler`.

### Audit Log page
The backend (`get_audit_log`, the `audit_logs` table) has existed
since Phase 1; this was purely the missing UI. `src/pages/AuditPage.tsx`
— result filter (success/failure/denied), free-text filter across
action/target/source, most recent 200 entries.

### Data retention (`commands::retention`)
- `RetentionSettings` (days to keep for `events`, `process_snapshots`,
  `port_snapshots`; `0` = keep forever), persisted the same way
  `NotificationSettings` already was — one JSON blob in the generic
  `settings` table.
- `run_retention_cleanup` deletes rows older than the configured
  window per table. Runs once automatically on every launch, and on
  demand from a "Clean up now" button on the Settings page.
- No `VACUUM` — SQLite in WAL mode reclaims freed pages into the
  freelist for reuse on its own; an explicit `VACUUM` would require
  an exclusive lock and rewrite the whole file, which isn't worth
  doing after every launch. Worth revisiting if the DB file size
  becomes a real complaint later.

## What's still open for Phase 6

**Testing.** No automated test suite exists yet — this pass added
source-level verification (checking Rust code against the actual
`windows` crate source, `tsc`/`vite build` for the frontend) but not
unit or integration tests. A reasonable minimum before calling this
phase done: `cargo test` coverage for the pure logic that doesn't
touch Windows APIs (`classify()` in `startup.rs`, the risk engine's
correlation rules, IPv4 validation in `dns.rs`), and a Playwright or
similar pass over the main page flows.

**Windows packaging.** `tauri.conf.json`'s bundle config
(identifier, icons, installer settings) hasn't been reviewed this
phase — worth a pass to confirm the NSIS/MSI output is actually
correct before a first public build, not just that `tauri build`
runs.

**A full Windows build.** Every phase's Windows-specific code has now
been checked against the actual `windows` crate source or, from Phase
4 onward, against real compiler output — but Scheduled Tasks
(`ITaskService`/`ITaskFolder`) is source-verified only, the same
starting point Phase 4's COM code was at before the BSTR/HSTRING bug
turned up. Treat it with the same suspicion until it's actually been
built and exercised on Windows.

## Debugging checklist for Scheduled Tasks specifically

1. `ITaskService::Connect` is called with four empty `VARIANT`s
   (default machine/user/domain/password, i.e. "connect locally as the
   current user") — confirm this is actually sufficient, or whether
   it needs to run elevated to see tasks registered by other users or
   SYSTEM.
2. `IRegisteredTask::Xml()` — confirm the `<Command>`/`<Arguments>`
   tag extraction actually matches real Task Scheduler XML output
   across task types (some actions are `<Exec>`, others `<ComHandler>`
   or `<SendEmail>` — the current extraction only handles `<Exec>`
   and will silently fall back to the task's registered path for
   anything else, which is a reasonable default but worth confirming
   in practice).
3. `remove_scheduled_task`'s re-walk-to-find-the-folder approach — if
   two tasks in different folders happen to share both name and exact
   command string, this matches the first one found. Unlikely in
   practice, but worth knowing about; storing the folder path
   alongside the entry in the DB would remove the ambiguity entirely
   if it turns out to matter.

## Handoff to whoever picks this up next

The core feature set from `ARCHITECTURE.md`'s phased plan is now
built. What's left is verification (a real Windows build, tests) and
polish (packaging) rather than new functionality — see "What's still
open" above.
