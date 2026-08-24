# Phase 5 Handoff — Scanning, Security Score, Notifications, Dashboard Polish

**Status:** Complete (pending the same Windows compile/debug pass as
every prior phase — see checklist at the end)
**Date:** 2026-08-23

## What shipped

### Scanning system (`commands::scan`)
- `run_scan(scan_type, custom_steps?)` — Quick / System / Network /
  Startup / Integrity / Custom, each mapped to a fixed subset of 7
  real steps (ports, firewall, startup, processes, services,
  adapters, files). Every step calls the *same* enumeration functions
  the rest of the app already uses (`ports::list_listening_ports`,
  `startup::list_startup_entries`, etc.) — no separate/duplicated
  scanning logic, and no fake progress: `step_index`/`total_steps` in
  the `scan-progress` event map 1:1 to actual work performed, emitted
  live as each step finishes.
- Findings are real too — e.g. the "startup" step flags anything not
  classified `Known` (reusing Phase 3's classification), the
  "services" step flags a *protected* service that isn't running.
  Nothing here invents a finding just to have something to show.
- Results persist to `scan_results` (schema already existed since
  Phase 1, unused until now) as `{ findings: [...], summary_text }`
  JSON in the `summary` column.
- `src/pages/ScansPage.tsx` — six scan cards (Custom has a checkbox
  picker for which steps to run), live progress bar during a run,
  expandable scan history.

### Security score (`commands::security_score`)
- `compute_security_score` — starts at 100, subtracts points from
  four concrete signals, each producing a `ScoreReason { label,
  impact, severity }` (never a bare number, per `SECURITY.md`):
  1. Firewall disabled for any profile (Domain/Private/Public) — read
     via `INetFwPolicy2::FirewallEnabled` (a COM *read*, unaffected by
     the BSTR bug from Phase 4 since it takes no string params).
  2. Suspicious/Unknown startup entries (reuses Phase 3's
     classification directly — the score and the Startup page can
     never disagree about what's suspicious).
  3. Open ports outside the Low-risk set (reuses Phase 2's port risk
     classification).
  4. High/Critical risk findings from the correlation engine in the
     last 7 days (reuses Phase 3's `risk_findings` table).
- Persists to a new `security_scores` table (history, for a future
  trend view) and is recomputed automatically every 10 minutes by a
  background task in `main.rs::setup`, emitting `security-score-updated`
  — the dashboard never needs to poll for this.
- `src/pages/SecurityPage.tsx` replaces the placeholder: score ring,
  full reasons breakdown, manual "Recalculate" button, link to Scans.
- Dashboard's Security Score stat card now shows the real score and
  its top reason instead of "Available from Phase 5".

### Notifications (`commands::notifications`)
- **Design decision worth flagging:** OS notification *dispatch*
  lives entirely in the frontend
  (`src/components/NotificationManager.tsx`), not in Rust. Rust only
  stores/returns the preference (`NotificationSettings { enabled,
  min_severity }`, JSON blob in the `settings` table under
  `notification_settings`). `NotificationManager` polls
  `get_recent_events` every 8s, compares against the last-seen
  timestamp and the stored severity threshold, and calls Tauri's
  `sendNotification` API directly for anything new that qualifies.
  This was chosen over wiring notification dispatch into
  `record_event` on the Rust side specifically to avoid a second,
  Rust-side OS-notification pathway that could drift out of sync with
  what the Events page already shows — one source of truth (the
  events table), one place that decides what's notification-worthy
  (the frontend, reading the same settings the Settings page writes).
- `src/pages/SettingsPage.tsx` — theme toggle (light/dark/system,
  already existed in `App.tsx`, now also editable here) +
  enabled/min-severity notification controls. Explicitly scoped as
  "not full configuration management yet" — retention, watch scopes,
  and scan scheduling stay Phase 6.

### Dashboard polish
- Security Score card wired to real data (see above).
- Recent Events empty-state copy updated (no longer says monitoring
  subsystems "come online in later phases" — they're live now).
- New **Health** page (`src/pages/HealthPage.tsx`): fuller CPU/RAM/
  per-disk/uptime/process-count view than the dashboard's summary
  cards, explicit about *not* showing battery/temperature/storage
  health since Windows doesn't expose those reliably across all
  hardware — no invented numbers.

## Known gaps / not done

- **Audit Log page** is still a placeholder — the backend
  (`commands::audit`) has been complete since Phase 1, this is purely
  a missing frontend page. Reasonable Phase 6 pickup, or sooner if
  it's wanted before then.
- `run_risk_analysis` (Phase 3) is still not triggered automatically
  anywhere — the "Run risk analysis" button on the Events page is
  still the only way to run it. Wiring it to fire after every scan
  would have been a natural Phase 5 addition; didn't make it into
  this pass.
- Security score history (the new `security_scores` table) isn't
  surfaced anywhere yet — no trend chart. The table exists so this is
  additive whenever it's wanted.
- `NotificationSettings` is intentionally minimal (on/off + one
  severity threshold) — no per-category filtering (e.g. "notify for
  firewall changes but not file events"). The original spec's section
  23 examples (HIGH RISK EVENT / NEW PORT / FIREWALL CHANGE / DNS
  CHANGE) all still show up if they clear the severity bar; there's
  just no way to silence one category specifically yet.

## Full debugging checklist for the Windows pass

Same situation as every prior phase. Phase 5 adds:

1. **`INetFwPolicy2::FirewallEnabled`** (`security_score.rs::windows_impl`)
   — verify the method signature (takes a `NET_FW_PROFILE_TYPE2`,
   returns `VARIANT_BOOL`) against the pinned `windows` 0.54 crate,
   same way Phase 4's `services.rs`/`network.rs` fixes were verified
   against real crate source — this file wasn't checked that way yet.
2. **`compute_security_score`'s SQL** — the `risk_findings` severity
   filter uses the `'"HIGH"'`/`'"CRITICAL"'` (JSON-quoted string)
   pattern already established in Phase 3's `risk.rs`; double-check
   it still matches once real data exists.
3. **`run_scan`'s cross-module calls** — it calls `list_firewall_rules`,
   `list_startup_entries`, `list_processes`, `list_services`,
   `list_network_adapters`, and `get_recent_file_events` directly
   (via `app.state::<T>()`), not through their own `#[tauri::command]`
   wrapping. This is a normal, supported Tauri pattern, but it means a
   signature change to any of those functions in a future pass needs
   to update `scan.rs` too — nothing enforces that link at compile
   time beyond the type checker itself.
4. **Tauri Notification permission on Windows** — `isPermissionGranted`/
   `requestPermission`/`sendNotification` from
   `@tauri-apps/api/notification` haven't been exercised on a real
   Windows machine in this project; confirm the permission prompt
   actually appears and notifications land in the Action Center.
5. Once compiling: run each of the 6 scan types and confirm findings
   look sane; toggle Windows Firewall off for one profile and confirm
   the Security page's score drops with the right reason; add an
   obviously-suspicious startup entry and confirm both the Startup
   page and the Security page agree it's flagged; turn notifications
   on with a low severity threshold and confirm a toast appears for a
   new event.

## Handoff to Phase 6

Phase 6 (testing, performance/retention, hardening, docs, Windows
packaging) is the last phase in the original plan. Concrete carry-over
items from this pass and earlier ones, in one place for whoever picks
it up:
- Audit Log page (gap noted above).
- `persist_and_diff`'s O(n²) recursion in `startup.rs` (flagged since
  Phase 3, still unfixed).
- Retention policy for `events`/`process_snapshots`/`port_snapshots`/
  `security_scores` (all grow unbounded right now).
- Scheduled Tasks as a startup-persistence source (flagged since
  Phase 3).
- User-configurable file-integrity watch scopes from the UI (add/remove
  — the table and read command exist, the write commands don't).
- Delayed-auto-start service startup type (flagged since Phase 2).
