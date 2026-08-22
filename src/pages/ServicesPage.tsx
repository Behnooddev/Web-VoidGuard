import { useEffect, useMemo, useState } from "react";
import { Search, RefreshCw, Play, Square, RotateCw, ShieldAlert } from "lucide-react";
import { changeServiceStartupType, controlService, listServices } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { AppError } from "@/types/process";
import type {
  ServiceAction,
  ServiceInfo,
  StartupType,
} from "@/types/system_control";

type PendingAction = { service: ServiceInfo; action: ServiceAction };

const STARTUP_LABELS: Record<StartupType, string> = {
  AUTOMATIC: "Automatic",
  AUTOMATIC_DELAYED: "Automatic (Delayed)",
  MANUAL: "Manual",
  DISABLED: "Disabled",
  UNKNOWN: "Unknown",
};

export default function ServicesPage() {
  const [services, setServices] = useState<ServiceInfo[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<AppError | null>(null);
  const [pending, setPending] = useState<PendingAction | null>(null);
  const [actionError, setActionError] = useState<AppError | null>(null);

  const refresh = () => {
    setLoading(true);
    listServices()
      .then((s) => {
        setServices(s);
        setLoadError(null);
      })
      .catch((e: AppError) => setLoadError(e))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    refresh();
  }, []);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return services;
    return services.filter(
      (s) =>
        s.name.toLowerCase().includes(q) || s.display_name.toLowerCase().includes(q)
    );
  }, [services, query]);

  const runAction = async () => {
    if (!pending) return;
    setActionError(null);
    try {
      await controlService({ service_name: pending.service.name, action: pending.action });
      setPending(null);
      refresh();
    } catch (e) {
      setActionError(e as AppError);
    }
  };

  const handleStartupTypeChange = async (service: ServiceInfo, startup_type: StartupType) => {
    try {
      await changeServiceStartupType({ service_name: service.name, startup_type });
      refresh();
    } catch {
      // Surfaced via a fresh load; the select reverts to the actual
      // value once refresh() completes.
      refresh();
    }
  };

  return (
    <div className="p-6 space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold">Services</h1>
          <p className="text-sm text-muted-foreground">
            {filtered.length} of {services.length} services
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
          placeholder="Search by service or display name..."
          className="w-full pl-9 pr-3 py-2 text-sm rounded-md border border-border bg-card focus:outline-none focus:ring-2 focus:ring-primary"
        />
      </div>

      <div className="rounded-lg border border-border bg-card overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-muted text-muted-foreground text-xs uppercase tracking-wide">
            <tr>
              <th className="text-left px-4 py-2 font-medium">Service</th>
              <th className="text-left px-4 py-2 font-medium">Status</th>
              <th className="text-left px-4 py-2 font-medium">Startup Type</th>
              <th className="text-left px-4 py-2 font-medium">Executable</th>
              <th className="px-4 py-2 text-right font-medium">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border">
            {filtered.map((s) => (
              <tr key={s.name} className="hover:bg-muted/50">
                <td className="px-4 py-2">
                  <div className="flex items-center gap-1.5">
                    <span className="font-medium">{s.display_name || s.name}</span>
                    {s.protected && (
                      <ShieldAlert
                        className="h-3.5 w-3.5 text-severity-medium shrink-0"
                        aria-label="Protected system service"
                      />
                    )}
                  </div>
                  <span className="text-xs text-muted-foreground">{s.name}</span>
                </td>
                <td className="px-4 py-2">
                  <StatusBadge status={s.status} />
                </td>
                <td className="px-4 py-2">
                  <select
                    value={s.startup_type}
                    onChange={(e) =>
                      handleStartupTypeChange(s, e.target.value as StartupType)
                    }
                    className="text-xs bg-transparent border border-border rounded-md px-2 py-1 focus:outline-none focus:ring-2 focus:ring-primary"
                  >
                    {(Object.keys(STARTUP_LABELS) as StartupType[])
                      .filter((k) => k !== "UNKNOWN")
                      .map((k) => (
                        <option key={k} value={k}>
                          {STARTUP_LABELS[k]}
                        </option>
                      ))}
                  </select>
                </td>
                <td
                  className="px-4 py-2 text-muted-foreground truncate max-w-xs text-xs"
                  title={s.executable ?? ""}
                >
                  {s.executable ?? "\u2014"}
                </td>
                <td className="px-4 py-2">
                  <div className="flex justify-end gap-1">
                    <button
                      disabled={s.status === "RUNNING"}
                      onClick={() => setPending({ service: s, action: "START" })}
                      className="text-severity-low hover:bg-severity-low/10 rounded-md p-1.5 transition-colors disabled:opacity-30 disabled:pointer-events-none"
                      title="Start"
                    >
                      <Play className="h-4 w-4" />
                    </button>
                    <button
                      disabled={s.status === "STOPPED"}
                      onClick={() => setPending({ service: s, action: "STOP" })}
                      className="text-severity-high hover:bg-severity-high/10 rounded-md p-1.5 transition-colors disabled:opacity-30 disabled:pointer-events-none"
                      title="Stop"
                    >
                      <Square className="h-4 w-4" />
                    </button>
                    <button
                      onClick={() => setPending({ service: s, action: "RESTART" })}
                      className="text-muted-foreground hover:bg-muted rounded-md p-1.5 transition-colors"
                      title="Restart"
                    >
                      <RotateCw className="h-4 w-4" />
                    </button>
                  </div>
                </td>
              </tr>
            ))}
            {filtered.length === 0 && !loading && !loadError && (
              <tr>
                <td colSpan={5} className="px-4 py-8 text-center text-muted-foreground">
                  No matching services.
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

function StatusBadge({ status }: { status: ServiceInfo["status"] }) {
  const styles: Record<ServiceInfo["status"], string> = {
    RUNNING: "text-severity-low bg-severity-low/10",
    STOPPED: "text-muted-foreground bg-muted",
    PAUSED: "text-severity-medium bg-severity-medium/10",
    START_PENDING: "text-severity-medium bg-severity-medium/10",
    STOP_PENDING: "text-severity-medium bg-severity-medium/10",
    UNKNOWN: "text-muted-foreground bg-muted",
  };
  return (
    <span className={cn("text-xs px-2 py-0.5 rounded-full", styles[status])}>
      {status.replace("_", " ")}
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
  const verbs: Record<ServiceAction, string> = {
    START: "start",
    STOP: "stop",
    RESTART: "restart",
  };
  const isProtected = pending.service.protected;

  return (
    <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-50" onClick={onCancel}>
      <div
        className="bg-card border border-border rounded-lg shadow-xl w-full max-w-sm p-5"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="font-semibold text-sm mb-1">
          {verbs[pending.action][0].toUpperCase() + verbs[pending.action].slice(1)}{" "}
          {pending.service.display_name || pending.service.name}?
        </h3>
        {isProtected && (
          <div className="rounded-md border border-severity-medium/40 bg-severity-medium/10 px-3 py-2 text-xs text-severity-medium mb-3 flex gap-2">
            <ShieldAlert className="h-4 w-4 shrink-0" />
            <span>
              This is a protected system service. {verbs[pending.action]}ing it can affect
              system stability. Proceed only if you're sure.
            </span>
          </div>
        )}
        <p className="text-sm text-muted-foreground mb-3">
          This will {verbs[pending.action]} the <code>{pending.service.name}</code> service.
        </p>
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
              isProtected ? "bg-severity-high" : "bg-primary"
            )}
          >
            {verbs[pending.action][0].toUpperCase() + verbs[pending.action].slice(1)}
          </button>
        </div>
      </div>
    </div>
  );
}
