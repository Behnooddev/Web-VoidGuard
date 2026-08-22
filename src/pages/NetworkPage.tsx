import { useEffect, useMemo, useState } from "react";
import { Search, RefreshCw, Octagon, Lock, Unlock, ShieldQuestion } from "lucide-react";
import { closePort, listListeningPorts, openPort, terminatePortOwner } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { AppError } from "@/types/process";
import type { ListeningPort, PortRuleRequest } from "@/types/ports";

type PendingAction =
  | { kind: "terminate"; port: ListeningPort }
  | { kind: "open" | "close"; port: ListeningPort };

export default function NetworkPage() {
  const [ports, setPorts] = useState<ListeningPort[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<AppError | null>(null);
  const [pending, setPending] = useState<PendingAction | null>(null);
  const [actionError, setActionError] = useState<AppError | null>(null);

  const refresh = () => {
    setLoading(true);
    listListeningPorts()
      .then((p) => {
        setPorts(p);
        setLoadError(null);
      })
      .catch((e: AppError) => setLoadError(e))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 6000);
    return () => clearInterval(interval);
  }, []);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return ports;
    return ports.filter(
      (p) =>
        String(p.port).includes(q) ||
        (p.process_name ?? "").toLowerCase().includes(q) ||
        p.local_address.toLowerCase().includes(q)
    );
  }, [ports, query]);

  const runAction = async () => {
    if (!pending) return;
    setActionError(null);
    try {
      if (pending.kind === "terminate") {
        if (pending.port.pid == null) throw notFoundError();
        await terminatePortOwner(pending.port.port, pending.port.pid);
      } else {
        const req: PortRuleRequest = {
          port: pending.port.port,
          protocol: pending.port.protocol,
          direction: "INBOUND",
        };
        if (pending.kind === "open") await openPort(req);
        else await closePort(req);
      }
      setPending(null);
      refresh();
    } catch (e) {
      setActionError(e as AppError);
    }
  };

  return (
    <div className="p-6 space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold">Network \u2014 Open Ports</h1>
          <p className="text-sm text-muted-foreground">
            {filtered.length} of {ports.length} listening endpoints
          </p>
        </div>
        <button
          onClick={refresh}
          className="flex items-center gap-2 text-sm px-3 py-1.5 rounded-md border border-border hover:bg-muted transition-colors"
        >
          <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
          Refresh
        </button>
      </div>

      {loadError && (
        <div className="rounded-md border border-severity-medium/40 bg-severity-medium/10 px-4 py-3 text-sm text-severity-medium">
          <p className="font-medium">{loadError.message}</p>
          {loadError.details && <p className="text-xs mt-1">{loadError.details}</p>}
        </div>
      )}

      <div className="relative max-w-sm">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search by port, process, or address..."
          className="w-full pl-9 pr-3 py-2 text-sm rounded-md border border-border bg-card focus:outline-none focus:ring-2 focus:ring-primary"
        />
      </div>

      <div className="rounded-lg border border-border bg-card overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-muted text-muted-foreground text-xs uppercase tracking-wide">
            <tr>
              <th className="text-left px-4 py-2 font-medium">Protocol</th>
              <th className="text-left px-4 py-2 font-medium">Local Address</th>
              <th className="text-left px-4 py-2 font-medium">Port</th>
              <th className="text-left px-4 py-2 font-medium">Process</th>
              <th className="text-left px-4 py-2 font-medium">PID</th>
              <th className="text-left px-4 py-2 font-medium">Status</th>
              <th className="text-left px-4 py-2 font-medium">Risk</th>
              <th className="px-4 py-2 text-right font-medium">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border">
            {filtered.map((p, i) => (
              <tr key={`${p.protocol}-${p.port}-${p.pid}-${i}`} className="hover:bg-muted/50">
                <td className="px-4 py-2 font-mono text-xs">{p.protocol}</td>
                <td className="px-4 py-2 text-muted-foreground">{p.local_address}</td>
                <td className="px-4 py-2 tabular-nums font-medium">{p.port}</td>
                <td className="px-4 py-2 truncate max-w-[14rem]" title={p.executable_path ?? ""}>
                  {p.process_name ?? "\u2014"}
                </td>
                <td className="px-4 py-2 tabular-nums text-muted-foreground">{p.pid ?? "\u2014"}</td>
                <td className="px-4 py-2 text-xs">{p.status}</td>
                <td className="px-4 py-2">
                  <RiskBadge risk={p.risk} />
                </td>
                <td className="px-4 py-2">
                  <div className="flex justify-end gap-1">
                    <button
                      disabled={p.pid == null}
                      onClick={() => setPending({ kind: "terminate", port: p })}
                      className="text-severity-high hover:bg-severity-high/10 rounded-md p-1.5 transition-colors disabled:opacity-30 disabled:pointer-events-none"
                      title="Terminate owning process"
                    >
                      <Octagon className="h-4 w-4" />
                    </button>
                    <button
                      onClick={() => setPending({ kind: "close", port: p })}
                      className="text-muted-foreground hover:bg-muted rounded-md p-1.5 transition-colors"
                      title="Block this port (Windows Firewall)"
                    >
                      <Lock className="h-4 w-4" />
                    </button>
                    <button
                      onClick={() => setPending({ kind: "open", port: p })}
                      className="text-severity-low hover:bg-severity-low/10 rounded-md p-1.5 transition-colors"
                      title="Explicitly allow this port (Windows Firewall)"
                    >
                      <Unlock className="h-4 w-4" />
                    </button>
                  </div>
                </td>
              </tr>
            ))}
            {filtered.length === 0 && !loading && !loadError && (
              <tr>
                <td colSpan={8} className="px-4 py-8 text-center text-muted-foreground">
                  No matching listening ports.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {pending && (
        <ConfirmDialog
          pending={pending}
          error={actionError}
          onCancel={() => {
            setPending(null);
            setActionError(null);
          }}
          onConfirm={runAction}
        />
      )}
    </div>
  );
}

function notFoundError(): AppError {
  return {
    code: "PID_UNKNOWN",
    message: "No owning process is known for this port.",
    details: null,
    recoverable: false,
  };
}

function RiskBadge({ risk }: { risk: ListeningPort["risk"] }) {
  const styles: Record<ListeningPort["risk"], string> = {
    LOW: "text-severity-low bg-severity-low/10",
    MEDIUM: "text-severity-medium bg-severity-medium/10",
    HIGH: "text-severity-high bg-severity-high/10",
    UNKNOWN: "text-muted-foreground bg-muted",
  };
  return (
    <span className={cn("inline-flex items-center gap-1 text-xs px-2 py-0.5 rounded-full", styles[risk])}>
      {risk === "UNKNOWN" && <ShieldQuestion className="h-3 w-3" />}
      {risk}
    </span>
  );
}

function ConfirmDialog({
  pending,
  error,
  onCancel,
  onConfirm,
}: {
  pending: PendingAction;
  error: AppError | null;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const copy = {
    terminate: {
      title: "Terminate owning process?",
      body: `This will forcibly end ${pending.port.process_name ?? "the process"} (PID ${pending.port.pid}) that is holding port ${pending.port.port}.`,
      confirmLabel: "Terminate",
      confirmClass: "bg-severity-high",
    },
    close: {
      title: "Block this port?",
      body: `A Windows Firewall rule will be added to block inbound traffic on ${pending.port.protocol} port ${pending.port.port}. This does not stop the process itself.`,
      confirmLabel: "Block Port",
      confirmClass: "bg-severity-medium",
    },
    open: {
      title: "Allow this port?",
      body: `A Windows Firewall rule will be added to explicitly allow inbound traffic on ${pending.port.protocol} port ${pending.port.port}.`,
      confirmLabel: "Allow Port",
      confirmClass: "bg-primary",
    },
  }[pending.kind];

  return (
    <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-50" onClick={onCancel}>
      <div
        className="bg-card border border-border rounded-lg shadow-xl w-full max-w-sm p-5"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="font-semibold text-sm mb-1">{copy.title}</h3>
        <p className="text-sm text-muted-foreground mb-3">{copy.body}</p>
        {error && (
          <div className="rounded-md border border-severity-high/40 bg-severity-high/10 px-3 py-2 text-xs text-severity-high mb-3">
            <p className="font-medium">{error.message}</p>
            {error.details && <p className="mt-0.5">{error.details}</p>}
          </div>
        )}
        <div className="flex justify-end gap-2">
          <button
            onClick={onCancel}
            className="px-3 py-1.5 text-sm rounded-md border border-border hover:bg-muted transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={onConfirm}
            className={cn(
              "px-3 py-1.5 text-sm rounded-md text-white hover:opacity-90 transition-opacity",
              copy.confirmClass
            )}
          >
            {copy.confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
