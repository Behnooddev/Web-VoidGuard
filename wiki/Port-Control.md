# Port Control

VoidGuard's Network page (`src/pages/NetworkPage.tsx`) shows every
listening TCP/UDP endpoint and lets you act on it. This page explains
exactly what each action does under the hood.

## Seeing what's running on a port

`list_listening_ports` reads the Windows IP Helper API's owner-PID
tables (`GetExtendedTcpTable` with `TCP_TABLE_OWNER_PID_ALL`,
`GetExtendedUdpTable` with `UDP_TABLE_OWNER_PID`) — the same data
`netstat -ano` shows you, read directly via the native API rather than
by shelling out to `netstat`. For each row it resolves the owning
PID's image name and full path via `QueryFullProcessImageNameW`.

Rows are given a coarse **risk** label (Low for well-known ports like
80/443/53/445/135/3389 and anything below 1024, Medium otherwise).
This is a starting signal only — see [[Security-Model]] on why
VoidGuard doesn't auto-classify things as malicious.

## Terminating the process behind a port

The **Terminate** action (red octagon icon) calls
`terminate_port_owner(port, pid)`, which is a thin wrapper around the
same process-kill logic used by the Process Manager page — it just
also logs the action as a port-specific one ("terminated the process
holding port 4444") so the audit trail reads clearly. Requires
confirmation in the UI every time; nothing ever kills a process
automatically.

## Opening / closing a port

The **lock** (block) and **unlock** (allow) actions call
`close_port` / `open_port` with a typed `PortRuleRequest { port,
protocol, direction }`. These create or remove a single, narrowly
scoped Windows Firewall rule named `VoidGuard - <PROTO> <PORT>
(<DIRECTION>)`, via the native `INetFwPolicy2` / `INetFwRule` COM
interfaces — the same mechanism the Windows Firewall control panel
itself uses internally. There is no `netsh advfirewall` shell call
anywhere in this path.

- **Close/block** removes any existing VoidGuard-created allow rule
  for that exact port+protocol+direction. It does *not* touch rules
  you created yourself outside VoidGuard, and it does not stop the
  process — the process keeps listening, it's just no longer
  reachable through the firewall.
- **Open/allow** creates an explicit allow rule scoped to exactly that
  port, protocol, and direction (currently inbound-only from the UI;
  the backend command supports outbound too).

Every open/close action writes an `audit_logs` row (`OPEN_PORT` /
`CLOSE_PORT`) and, on success, a `FIREWALL_CHANGED` event.

## Known caveat

This module's native Windows code has not yet been compiled on an
actual Windows machine (it was written in a Linux development
sandbox). See `handoffs/02-phase-2-handoff.md` for the specific
structs/APIs that need verification against the pinned `windows`
crate version before this ships.
