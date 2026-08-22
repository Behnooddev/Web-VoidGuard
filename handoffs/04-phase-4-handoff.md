# Phase 4 Handoff — Firewall Rule Management, DNS

**Status:** Feature-complete for the phase scope (full rule CRUD +
per-interface DNS), pending Windows compile/debug pass — same
situation as every prior phase, see checklist below.
**Date:** 2026-08-22

## What shipped

### Firewall rule management (`commands::firewall`)
- `create_firewall_rule` / `set_firewall_rule_enabled` /
  `delete_firewall_rule` / `list_firewall_rules` — full CRUD via COM
  (`INetFwPolicy2`/`INetFwRule`), generalizing the single-port
  open/close already shipped in Phase 2 (`commands::ports::set_port_rule`,
  left untouched).
- **Design decision worth flagging:** `list_firewall_rules` does *not*
  enumerate the entire Windows Firewall rule set. That would mean
  walking `INetFwRules` via its `_NewEnum`/`IEnumVARIANT` COM
  enumerator — real, but low-level, poorly-covered-by-examples COM
  automation that I couldn't verify precisely against the pinned
  `windows` crate version without a Windows toolchain, and even if
  correct it'd surface the *hundreds* of built-in Windows/app rules
  already on a typical machine, which isn't useful in this UI anyway.
  Instead, VoidGuard now tracks the rules **it itself created** in the
  local `firewall_rules` table (already present in the schema since
  Phase 1, just unused) and only ever calls the COM API by exact rule
  name (`Rules().Item(name)` / `.Remove(name)`) — both of which are
  simple, well-documented, single-purpose calls in the same family as
  `Add`/`Remove` that Phase 2's `set_port_rule` already uses
  successfully. This is the same "small, scoped, honest about limits"
  approach the file-integrity watcher took in Phase 3 rather than
  pretending to cover more than it verifiably does.
- `src/pages/FirewallPage.tsx` replaces the placeholder: rule table,
  create-rule dialog (name/description/action/direction/protocol/local
  ports/remote addresses/application path/enabled), enable-disable
  toggle, delete — all confirmed via dialog first, same pattern as
  Startup/Network pages.
- DB: `firewall_rules` gained `description` and `remote_addresses`
  columns, and `name` is now `UNIQUE` (rules are upserted by name).

### Per-interface DNS (`commands::dns`)
- `change_dns` — sets or clears a per-interface DNS override via the
  native `SetInterfaceDnsSettings` API (`netioapi.h`), never `netsh`
  or a shell, per `SECURITY.md`. Takes a `DnsSettingsRequest` keyed by
  adapter GUID (`NetworkAdapter::adapter_id`, newly added — see
  below), never a display name.
- Server-side IPv4 validation (rejects malformed and leading-zero
  octets) before anything reaches the Windows API; DHCP mode sends an
  empty `NameServer` list, which per Microsoft's docs reverts the
  interface to automatic DNS — **not yet confirmed against real
  Windows behavior**, see checklist.
- `commands::network::list_network_adapters` now also returns each
  adapter's GUID (`adapter_id`, from the ANSI `AdapterName` field,
  distinct from the wide-string `FriendlyName`/`Description` already
  parsed) so the frontend has something stable to target — adapter
  *names* aren't guaranteed unique or stable enough for this.
- UI lives in `AdaptersTab.tsx` (Network → Adapters), not a separate
  page: each adapter card's DNS row got an edit affordance opening a
  DHCP/Static dialog, since DNS is adapter-scoped info the page
  already displays read-only.

### Incidental fixes (not phase 4 work, found while building)
- `vite.config.ts` had no `resolve.alias` for `@/*`, even though
  `tsconfig.json` defines that path mapping. `tsc` respects
  `tsconfig.json` paths on its own (which is why `tsc --noEmit` was
  passing), but Vite/Rollup only understand `resolve.alias` — so
  `npm run build` was silently broken from Phase 1 onward, just never
  actually run until now. Fixed with a one-line alias mirroring the
  tsconfig path; `npx vite build` now succeeds end-to-end (1507
  modules, clean).
- `terminateProcess` in `lib/ipc.ts` had a pre-existing `tsc` error
  (`Promise<unknown>` not assignable to `Promise<void>`) from an
  untyped `invoke()` call — fixed with an explicit `invoke<void>`.

## Full debugging checklist for the Windows pass

Same situation as every prior phase — written without a Windows
compiler available (see Phase 2/3 handoffs' checklists, still valid).
Phase 4 adds:

1. **`DNS_INTERFACE_SETTINGS` field-for-field check**
   (`commands/dns.rs::windows_impl`) — the struct fields, the
   `PWSTR`/`Version` constant names, and `SetInterfaceDnsSettings`'s
   exact return type (assumed `WIN32_ERROR`, compared against
   `NO_ERROR`) should all be checked against whatever `windows` crate
   version ends up pinned in `Cargo.lock` — this API has fewer public
   usage examples than the firewall/IpHelper calls from earlier
   phases, so it's the least-verified code in this pass.
2. **Confirm DHCP fallback behavior** — does an empty `NameServer` on
   `SetInterfaceDnsSettings` actually revert the interface to
   DHCP-assigned DNS, or does it need a distinct flag/flow? Test by
   setting a static DNS, then switching a test adapter back to
   "Automatic (DHCP)" in the UI and checking `ipconfig /all`.
3. **`NET_FW_IP_PROTOCOL_ANY`** (`commands/firewall.rs::windows_impl`)
   is hand-defined as `256` since it wasn't obviously exposed as a
   named constant in the `WindowsFirewall` bindings — verify it's
   still needed once the actual crate version is checked out (it may
   already be exported under a different name).
4. **`INetFwRule::SetRemotePorts` / `SetRemoteAddresses` /
   `SetApplicationName`** — used by analogy with the already-proven
   `SetLocalPorts`/`SetProtocol`/`SetDirection` calls from Phase 2, but
   not independently exercised yet.
5. Once compiling: create a firewall rule from the UI, confirm it
   shows up in the Windows Firewall control panel (`wf.msc`) with the
   expected scope; toggle enabled/disabled and confirm both UI and
   `wf.msc` agree; delete it and confirm it's gone from both places
   and from the `firewall_rules` table; change a test adapter's DNS to
   a static pair and confirm `ipconfig /all` reflects it, then switch
   back to DHCP.

## Post-phase-4 build fixes (services.rs / network.rs)

After this phase shipped, a real attempt to run `npm run tauri dev`
surfaced actual Rust compile errors in the Phase 1/2 code
(`services.rs`, `network.rs`) — the first time any of this project's
Windows-specific code had been checked against a real toolchain. Fixed
by downloading the actual pinned `windows 0.54.0` crate source from
crates.io and verifying every symbol against it directly, rather than
guessing:

- `SC_HANDLE` is exported from `windows::Win32::Security`, not
  `windows::Win32::System::Services`.
- `ENUM_SERVICE_STATE` and `GET_ADAPTERS_ADDRESSES_FLAGS` don't derive
  `BitOr` in this crate — `SERVICE_ACTIVE | SERVICE_INACTIVE` and
  `GAA_FLAG_INCLUDE_PREFIX | GAA_FLAG_INCLUDE_GATEWAYS` are both
  replaced (the former with the crate's own `SERVICE_STATE_ALL`
  constant, the latter with a manually-OR'd, re-wrapped value).
- `dwCurrentState`/`dwStartType` are typed
  (`SERVICE_STATUS_CURRENT_STATE`/`SERVICE_START_TYPE`), not `u32` —
  fixed the comparisons in `map_status` and the startup-type match.
- `SERVICE_NO_CHANGE` is a bare `u32`; `ChangeServiceConfigW`'s
  `dwServiceType`/`dwErrorControl` params are typed — now wrapped as
  `ENUM_SERVICE_TYPE`/`SERVICE_ERROR`.
- The service-restart match was non-exhaustive (missing the "stop
  succeeded, start failed" case) — added, with its own message.
- `IP_ADAPTER_ADDRESSES_LH::Flags` doesn't exist at the top level in
  this crate's bindings — it's `adapter.Anonymous2.Flags`.
- `ports.rs` was checked against the same crate source and had no
  actual bugs.

**On the reported "multiple windows-core versions" issue:** installed
`rustc`/`cargo` (from Ubuntu's apt, since no internet access to
rustup) and ran `cargo generate-lockfile` against the real crates.io
index to see the actual resolved dependency graph. Confirmed this is
**not a bug**: `rfd` pins `windows 0.37`, `tao`/`tauri`/`wry`/
`webview2-com` pin `windows 0.39`, `generator` pins `windows 0.48`,
`sysinfo` pins `windows 0.52`, and `iana-time-zone` (a transitive dep
of `chrono`) pins `windows-core 0.62.2` — five completely independent,
self-contained usages that never exchange types with our own
`windows 0.54` code or each other. Multiple incompatible major
versions of a pre-1.0 crate coexisting in one dependency graph is
normal Cargo behavior, not a conflict, as long as nothing crosses
those boundaries — which nothing here does. The actual
`HSTRING`/`BSTR`/`IntoParam` errors reported were symptoms of the real
type bugs above, not of version fragmentation.

**Caveat, stated plainly:** none of this was verified by an actual
successful Windows compile — no Windows machine or MSVC/GNU Windows
target was available in this environment either, only Linux with a
Rust toolchain installed from `apt` (1.75, old enough that
`cargo tree` itself failed against some current crates' manifests,
though `cargo generate-lockfile` succeeded). Every fix above was
checked line-by-line against the real, downloaded `windows-0.54.0`
crate source (struct field names, parameter types, exact constant
values) rather than guessed from memory or pattern-matched from
similar APIs — but the only way to be fully sure is still a real
`cargo build --target x86_64-pc-windows-msvc` on Windows, which
remains item #1 for whoever does the next Windows pass.

## Handoff to Phase 5

Phase 5 owner: quick/system/network/startup/integrity/custom scans
(`commands::scan`), the aggregate security-scoring engine
(`commands::security_score`) — the risk engine's findings from Phase 3
are a natural input here — and user-configurable OS notifications
(`commands::notifications`). `ARCHITECTURE.md`'s module table has the
full breakdown. The `run_risk_analysis` "not yet scheduled
automatically" note from Phase 3 is also still open — Phase 5 is a
reasonable place to trigger it (e.g. after every scan).
