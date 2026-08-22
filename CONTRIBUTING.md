# Contributing to VoidGuard

VoidGuard is a local, defensive Windows security/monitoring tool.
Contributions are welcome — please read this before opening a PR.

## Before you start

- Check `ARCHITECTURE.md` for the module map and current phase status,
  and the `handoffs/` folder for the most recent handoff doc — it
  lists exactly what's done, what's in progress, and known caveats.
- Check open issues/PRs so you're not duplicating work.
- For anything non-trivial, open an issue first to agree on approach
  before writing code.

## Hard rules (see `SECURITY.md` for full detail)

PRs that add any of the following will be closed, no exceptions:

- Arbitrary shell / `cmd.exe` / PowerShell execution, or any command
  that accepts a free-form string and passes it to a process-spawn API
- Remote control, remote shell, or covert-channel functionality of any
  kind
- Hidden/undocumented persistence
- Credential collection, keystroke logging, clipboard scraping
- Anything that disables or bypasses Windows security features
- Anything that hides the app's own process/files/network activity

Every privileged operation must be a typed `#[tauri::command]`, must
validate its input, and must call `commands::audit::record_audit` on
both success and failure paths.

## Setup

See `DEVELOPMENT.md`.

## The pattern for adding a subsystem

Follow the five-step recipe in `DEVELOPMENT.md` under "Adding a new
subsystem" — it's the same shape used by every existing module
(`events`, `audit`, `process`, `ports`). Copy that shape rather than
inventing a new one.

## Commit / PR conventions

- Keep PRs scoped to one subsystem or one fix.
- `cargo fmt` and `cargo clippy -- -D warnings` must pass.
- If your PR completes or meaningfully advances a phase from
  `ARCHITECTURE.md`, add or update the corresponding file in
  `handoffs/` (see existing ones for the expected sections: what
  shipped, files of note, known limitations, verification needed,
  handoff to next phase).
- Update `ARCHITECTURE.md`'s module status table in the same PR.

## Reporting security concerns

If you find a code path that violates the constraints in
`SECURITY.md`, please open an issue tagged `security` rather than a
silent PR — we want a record of what was found and why it was wrong,
not just a quiet fix.
