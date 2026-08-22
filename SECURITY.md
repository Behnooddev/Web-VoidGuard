# Security Model

VoidGuard is a **local, defensive** system-monitoring and
administration tool. This document states what it will and will not
do, and how the privilege boundary is enforced in code — not just in
policy.

## Hard constraints (enforced, not aspirational)

VoidGuard does **not** implement, and any contribution that adds any
of the following should be rejected in review:

- Arbitrary shell / `cmd.exe` / PowerShell execution
- Any IPC command that accepts a free-form command string and passes
  it to a process spawn API
- Remote command-and-control, remote shell, or covert channel of any
  kind
- Hidden or undocumented persistence mechanisms
- Credential collection, keystroke logging, or clipboard scraping
- Disabling or bypassing Windows Defender / other security software
- Any mechanism to hide the app's own process, files, or network
  activity from the OS or from the user

## The typed-command boundary

The frontend can only call the exact set of `#[tauri::command]`
functions registered in `main.rs`'s `invoke_handler!`. Each one takes
a strongly typed argument (deserialized by `serde`, validated by
handwritten checks) — never a raw string that gets interpreted as a
command. Example shape used throughout the backend:

```rust
#[derive(Deserialize)]
struct ChangeDnsRequest {
    interface_id: String,
    primary_dns: IpAddr,
    secondary_dns: Option<IpAddr>,
}

#[tauri::command]
fn change_dns(req: ChangeDnsRequest, db: State<Db>) -> Result<AuditEntry, AppError> {
    // 1. validate req against current adapter list
    // 2. call the Windows API (no shelling out)
    // 3. record_audit(...) unconditionally, success or failure
}
```

If a Windows capability cannot be safely reached this way, the
command returns `AppError::not_supported(...)` and the UI shows that
state honestly — it does not fabricate success.

## Privilege handling

- The app runs with standard-user privileges by default.
- Operations that require elevation (some firewall/service/DNS
  changes) trigger an explicit Windows elevation prompt scoped to
  that single operation — the app does not hold a permanent elevated
  token.
- Read-only monitoring (metrics, process listing, event viewing) never
  requires elevation.

## Auditability

Every privileged/administrative action — regardless of outcome —
produces one row in the append-only `audit_logs` table: actor, action,
target, before/after state, result, source, and app version. The
audit log is written from the backend only; there is no code path
that lets the UI skip it.

## Destructive-action confirmation

Process termination, service stop/restart on protected services,
firewall rule deletion, and DNS changes all require an explicit
confirmation dialog in the UI *before* the backend command is
invoked. None of these actions fire silently or automatically from a
background scan.

## Detection philosophy

The detection/risk engine (Phases 3 and 5) reports **evidence and
confidence**, not verdicts. Unknown or unsigned software is flagged
with the reasons for suspicion — it is never auto-labeled "malware,"
auto-quarantined, or auto-deleted. All remediation is a suggestion
the user acts on explicitly.

## Reporting a concern

If you find a code path that violates any constraint above, treat it
as a bug: file it, and it should be fixed or the feature removed
before release.
