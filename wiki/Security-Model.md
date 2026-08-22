# Security Model

Condensed from the repo's `SECURITY.md` — read that file for the full
version, including code examples.

## What VoidGuard will never do

- Arbitrary shell / `cmd.exe` / PowerShell execution
- Accept a free-form command string from the UI and spawn it
- Remote control, remote shell, or covert channel of any kind
- Hidden/undocumented persistence
- Credential collection, keystroke logging, clipboard scraping
- Disable or bypass Windows Defender or other security software
- Hide its own process, files, or network activity

## How privileged actions work

Every mutating action (kill a process, open/close a port, later:
change DNS, edit firewall rules, start/stop services) is:

1. A named, typed Tauri command — never a raw string sent to the OS.
2. Confirmed explicitly in the UI *before* the backend is even called,
   for anything destructive.
3. Validated again server-side (never trust the frontend).
4. Executed via a native Windows API or COM interface — no `netsh`,
   no shell.
5. Recorded in the append-only `audit_logs` table, success or
   failure, with before/after state.

## Detection philosophy

Unknown or unsigned software is reported with evidence and a
confidence level — never auto-labeled "malware," auto-quarantined, or
auto-deleted. All remediation is a suggestion the user acts on
explicitly.

## Elevation

The app runs as a standard user by default. Operations that need
elevation trigger a scoped Windows elevation prompt for that single
operation — VoidGuard does not hold a permanent elevated token.

## Reporting a concern

Use the **Security concern** issue template in the repo, or see
`CONTRIBUTING.md` for private-disclosure guidance on live
vulnerabilities.
