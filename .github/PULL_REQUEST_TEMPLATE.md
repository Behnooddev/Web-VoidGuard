## What this PR does

## Which phase/module (see ARCHITECTURE.md)

## Checklist

- [ ] No arbitrary shell/PowerShell/cmd execution introduced (see `SECURITY.md`)
- [ ] Every privileged command validates its input and calls `record_audit` on both success and failure
- [ ] Destructive actions require explicit UI confirmation before the backend command is invoked
- [ ] `cargo fmt` and `cargo clippy -- -D warnings` pass
- [ ] Tests added/updated (`cargo test`) where applicable
- [ ] `ARCHITECTURE.md` module status table updated if this changes it
- [ ] If this completes or meaningfully advances a phase, added/updated a `handoffs/NN-phase-N-handoff.md`

## Screenshots (for UI changes)
