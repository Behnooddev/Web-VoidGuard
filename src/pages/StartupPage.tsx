import { useEffect, useState } from "react";
import { RefreshCw, Trash2, ShieldQuestion, ShieldAlert, ShieldCheck } from "lucide-react";
import { listStartupEntries, removeStartupEntry } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { AppError } from "@/types/process";
import type { StartupClassification, StartupEntry } from "@/types/monitoring";

const LOCATION_LABELS: Record<StartupEntry["location_type"], string> = {
  REGISTRY_RUN: "Registry (Run)",
  REGISTRY_RUN_ONCE: "Registry (RunOnce)",
  STARTUP_FOLDER: "Startup folder",
  SCHEDULED_TASK: "Scheduled task",
};

export default function StartupPage() {
  const [entries, setEntries] = useState<StartupEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<AppError | null>(null);
  const [pending, setPending] = useState<StartupEntry | null>(null);
  const [actionError, setActionError] = useState<AppError | null>(null);

  const refresh = () => {
    setLoading(true);
    listStartupEntries()
      .then((e) => {
        setEntries(e);
        setLoadError(null);
      })
      .catch((e: AppError) => setLoadError(e))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    refresh();
  }, []);

  const handleRemove = async () => {
    if (!pending) return;
    setActionError(null);
    try {
      await removeStartupEntry(pending.id);
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
          <h1 className="text-xl font-semibold">Startup</h1>
          <p className="text-sm text-muted-foreground">
            Registry Run/RunOnce keys and Startup folders — {entries.length} entries
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

      <p className="text-xs text-muted-foreground rounded-md border border-border bg-card px-3 py-2">
        Scheduled Tasks aren't scanned yet — this page currently covers
        registry Run/RunOnce keys and Startup folders only.
      </p>

      {loadError && (
        <div className="rounded-md border border-severity-medium/40 bg-severity-medium/10 px-4 py-3 text-sm text-severity-medium">
          <p className="font-medium">{loadError.message}</p>
          {loadError.details && <p className="text-xs mt-1">{loadError.details}</p>}
        </div>
      )}

      <div className="rounded-lg border border-border bg-card overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-muted text-muted-foreground text-xs uppercase tracking-wide">
            <tr>
              <th className="text-left px-4 py-2 font-medium">Name</th>
              <th className="text-left px-4 py-2 font-medium">Command</th>
              <th className="text-left px-4 py-2 font-medium">Location</th>
              <th className="text-left px-4 py-2 font-medium">Classification</th>
              <th className="px-4 py-2 text-right font-medium">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border">
            {entries.map((e) => (
              <tr key={e.id} className="hover:bg-muted/50 align-top">
                <td className="px-4 py-2 font-medium">{e.name}</td>
                <td className="px-4 py-2 font-mono text-xs text-muted-foreground truncate max-w-xs" title={e.command}>
                  {e.command}
                </td>
                <td className="px-4 py-2 text-xs">{LOCATION_LABELS[e.location_type]}</td>
                <td className="px-4 py-2">
                  <ClassificationBadge classification={e.classification} evidence={e.evidence} />
                </td>
                <td className="px-4 py-2 text-right">
                  <button
                    onClick={() => setPending(e)}
                    className="text-severity-high hover:bg-severity-high/10 rounded-md p-1.5 transition-colors"
                    title="Remove startup entry"
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                </td>
              </tr>
            ))}
            {entries.length === 0 && !loading && !loadError && (
              <tr>
                <td colSpan={5} className="px-4 py-8 text-center text-muted-foreground">
                  No startup entries found.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {pending && (
        <ConfirmDialog
          entry={pending}
          error={actionError}
          onCancel={() => {
            setPending(null);
            setActionError(null);
          }}
          onConfirm={handleRemove}
        />
      )}
    </div>
  );
}

function ClassificationBadge({
  classification,
  evidence,
}: {
  classification: StartupClassification;
  evidence: string[];
}) {
  const config: Record<StartupClassification, { icon: typeof ShieldCheck; cls: string }> = {
    KNOWN: { icon: ShieldCheck, cls: "text-severity-low bg-severity-low/10" },
    UNKNOWN: { icon: ShieldQuestion, cls: "text-muted-foreground bg-muted" },
    SUSPICIOUS: { icon: ShieldAlert, cls: "text-severity-high bg-severity-high/10" },
  };
  const { icon: Icon, cls } = config[classification];
  return (
    <div className="group relative inline-block">
      <span className={cn("inline-flex items-center gap-1 text-xs px-2 py-0.5 rounded-full cursor-help", cls)}>
        <Icon className="h-3 w-3" />
        {classification}
      </span>
      {evidence.length > 0 && (
        <div className="hidden group-hover:block absolute z-10 top-full left-0 mt-1 w-64 rounded-md border border-border bg-card shadow-lg p-2 text-xs text-muted-foreground">
          <ul className="list-disc list-inside space-y-0.5">
            {evidence.map((ev, i) => (
              <li key={i}>{ev}</li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

function ConfirmDialog({
  entry,
  error,
  onCancel,
  onConfirm,
}: {
  entry: StartupEntry;
  error: AppError | null;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-50" onClick={onCancel}>
      <div
        className="bg-card border border-border rounded-lg shadow-xl w-full max-w-sm p-5"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="font-semibold text-sm mb-1">Remove startup entry?</h3>
        <p className="text-sm text-muted-foreground mb-3">
          <span className="font-medium text-foreground">{entry.name}</span> will no longer run
          automatically. This does not remove or uninstall the program itself.
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
            className="px-3 py-1.5 text-sm rounded-md bg-severity-high text-white hover:opacity-90 transition-opacity"
          >
            Remove
          </button>
        </div>
      </div>
    </div>
  );
}
