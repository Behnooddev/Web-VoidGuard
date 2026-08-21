import { useEffect, useMemo, useState } from "react";
import { Search, RefreshCw, Octagon, ShieldAlert, ShieldCheck } from "lucide-react";
import { listProcesses, terminateProcess } from "@/lib/ipc";
import { cn, formatBytes } from "@/lib/utils";
import type { ProcessInfo, AppError } from "@/types/process";

type SortKey = "name" | "pid" | "cpu_percent" | "memory_bytes";

export default function ProcessesPage() {
  const [processes, setProcesses] = useState<ProcessInfo[]>([]);
  const [query, setQuery] = useState("");
  const [sortKey, setSortKey] = useState<SortKey>("cpu_percent");
  const [sortDesc, setSortDesc] = useState(true);
  const [loading, setLoading] = useState(true);
  const [confirmTarget, setConfirmTarget] = useState<ProcessInfo | null>(null);
  const [actionError, setActionError] = useState<AppError | null>(null);

  const refresh = () => {
    setLoading(true);
    listProcesses()
      .then(setProcesses)
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 4000);
    return () => clearInterval(interval);
  }, []);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    let list = processes;
    if (q) {
      list = list.filter(
        (p) =>
          p.name.toLowerCase().includes(q) ||
          String(p.pid).includes(q) ||
          (p.exe_path ?? "").toLowerCase().includes(q)
      );
    }
    return [...list].sort((a, b) => {
      const dir = sortDesc ? -1 : 1;
      if (sortKey === "name") return dir * a.name.localeCompare(b.name);
      return dir * ((a[sortKey] as number) - (b[sortKey] as number));
    });
  }, [processes, query, sortKey, sortDesc]);

  const toggleSort = (key: SortKey) => {
    if (key === sortKey) setSortDesc((d) => !d);
    else {
      setSortKey(key);
      setSortDesc(true);
    }
  };

  const handleTerminate = async () => {
    if (!confirmTarget) return;
    setActionError(null);
    try {
      await terminateProcess(confirmTarget.pid);
      setConfirmTarget(null);
      refresh();
    } catch (e) {
      setActionError(e as AppError);
    }
  };

  return (
    <div className="p-6 space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold">Processes</h1>
          <p className="text-sm text-muted-foreground">
            {filtered.length} of {processes.length} processes
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

      <div className="relative max-w-sm">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search by name, PID, or path..."
          className="w-full pl-9 pr-3 py-2 text-sm rounded-md border border-border bg-card focus:outline-none focus:ring-2 focus:ring-primary"
        />
      </div>

      <div className="rounded-lg border border-border bg-card overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-muted text-muted-foreground text-xs uppercase tracking-wide">
            <tr>
              <Th label="Name" onClick={() => toggleSort("name")} />
              <Th label="PID" onClick={() => toggleSort("pid")} />
              <Th label="CPU" onClick={() => toggleSort("cpu_percent")} />
              <Th label="Memory" onClick={() => toggleSort("memory_bytes")} />
              <th className="text-left px-4 py-2 font-medium">Path</th>
              <th className="text-left px-4 py-2 font-medium">Signature</th>
              <th className="px-4 py-2" />
            </tr>
          </thead>
          <tbody className="divide-y divide-border">
            {filtered.map((p) => (
              <tr key={p.pid} className="hover:bg-muted/50">
                <td className="px-4 py-2 font-medium">{p.name}</td>
                <td className="px-4 py-2 tabular-nums text-muted-foreground">{p.pid}</td>
                <td className="px-4 py-2 tabular-nums">{p.cpu_percent.toFixed(1)}%</td>
                <td className="px-4 py-2 tabular-nums">{formatBytes(p.memory_bytes)}</td>
                <td className="px-4 py-2 text-muted-foreground truncate max-w-xs" title={p.exe_path ?? ""}>
                  {p.exe_path ?? "\u2014"}
                </td>
                <td className="px-4 py-2">
                  <SignatureBadge signed={p.signed} />
                </td>
                <td className="px-4 py-2 text-right">
                  <button
                    onClick={() => setConfirmTarget(p)}
                    className="text-severity-high hover:bg-severity-high/10 rounded-md p-1.5 transition-colors"
                    title="Terminate process"
                  >
                    <Octagon className="h-4 w-4" />
                  </button>
                </td>
              </tr>
            ))}
            {filtered.length === 0 && !loading && (
              <tr>
                <td colSpan={7} className="px-4 py-8 text-center text-muted-foreground">
                  No matching processes.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {confirmTarget && (
        <ConfirmDialog
          process={confirmTarget}
          error={actionError}
          onCancel={() => {
            setConfirmTarget(null);
            setActionError(null);
          }}
          onConfirm={handleTerminate}
        />
      )}
    </div>
  );
}

function Th({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <th className="text-left px-4 py-2 font-medium cursor-pointer select-none" onClick={onClick}>
      {label}
    </th>
  );
}

function SignatureBadge({ signed }: { signed: boolean | null }) {
  if (signed === null) {
    return <span className="text-xs text-muted-foreground">Unknown</span>;
  }
  return signed ? (
    <span className="inline-flex items-center gap-1 text-xs text-severity-low">
      <ShieldCheck className="h-3.5 w-3.5" /> Signed
    </span>
  ) : (
    <span className="inline-flex items-center gap-1 text-xs text-severity-medium">
      <ShieldAlert className="h-3.5 w-3.5" /> Unsigned
    </span>
  );
}

function ConfirmDialog({
  process,
  error,
  onCancel,
  onConfirm,
}: {
  process: ProcessInfo;
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
        <h3 className="font-semibold text-sm mb-1">Terminate process?</h3>
        <p className="text-sm text-muted-foreground mb-3">
          This will forcibly end <span className="font-medium text-foreground">{process.name}</span> (PID{" "}
          {process.pid}). Unsaved work in this process will be lost.
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
            Terminate
          </button>
        </div>
      </div>
    </div>
  );
}
