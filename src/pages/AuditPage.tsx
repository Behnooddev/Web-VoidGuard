import { useEffect, useState } from "react";
import { RefreshCw, ShieldCheck, ShieldAlert, ShieldX } from "lucide-react";
import { getAuditLog } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import EmptyState from "@/components/EmptyState";
import type { AuditEntry, AuditResult } from "@/types";

const RESULT_FILTERS: { label: string; value: AuditResult | null }[] = [
  { label: "All", value: null },
  { label: "Success", value: "SUCCESS" },
  { label: "Failure", value: "FAILURE" },
  { label: "Denied", value: "DENIED" },
];

export default function AuditPage() {
  const [entries, setEntries] = useState<AuditEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [resultFilter, setResultFilter] = useState<AuditResult | null>(null);
  const [query, setQuery] = useState("");

  const refresh = () => {
    setLoading(true);
    getAuditLog(200)
      .then(setEntries)
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    refresh();
  }, []);

  const filtered = entries.filter((e) => {
    if (resultFilter && e.result !== resultFilter) return false;
    if (!query.trim()) return true;
    const q = query.toLowerCase();
    return (
      e.action.toLowerCase().includes(q) ||
      e.target.toLowerCase().includes(q) ||
      e.source.toLowerCase().includes(q)
    );
  });

  return (
    <div className="p-6 space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold">Audit Log</h1>
          <p className="text-sm text-muted-foreground">
            Every privileged action VoidGuard has taken, success or failure
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

      <div className="flex items-center gap-3 flex-wrap">
        <div className="flex gap-1">
          {RESULT_FILTERS.map((f) => (
            <button
              key={f.label}
              onClick={() => setResultFilter(f.value)}
              className={cn(
                "text-xs px-3 py-1.5 rounded-full border transition-colors",
                resultFilter === f.value
                  ? "border-primary text-primary bg-accent"
                  : "border-border text-muted-foreground hover:bg-muted"
              )}
            >
              {f.label}
            </button>
          ))}
        </div>
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Filter by action, target, or source…"
          className="flex-1 min-w-[200px] px-3 py-1.5 text-sm rounded-md border border-border bg-background focus:outline-none focus:ring-2 focus:ring-primary"
        />
      </div>

      <div className="rounded-lg border border-border bg-card overflow-hidden">
        {filtered.length === 0 ? (
          <EmptyState
            title="No matching entries"
            description={
              entries.length === 0
                ? "Nothing has been logged yet — privileged actions appear here as they happen."
                : "Nothing matches this filter."
            }
          />
        ) : (
          <table className="w-full text-sm">
            <thead className="bg-muted text-muted-foreground text-xs uppercase tracking-wide">
              <tr>
                <th className="text-left px-4 py-2 font-medium">Result</th>
                <th className="text-left px-4 py-2 font-medium">Action</th>
                <th className="text-left px-4 py-2 font-medium">Target</th>
                <th className="text-left px-4 py-2 font-medium">Change</th>
                <th className="text-left px-4 py-2 font-medium">Source</th>
                <th className="text-left px-4 py-2 font-medium">When</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              {filtered.map((e) => (
                <tr key={e.id} className="hover:bg-muted/50 align-top">
                  <td className="px-4 py-2">
                    <ResultBadge result={e.result} />
                  </td>
                  <td className="px-4 py-2 font-mono text-xs">{e.action}</td>
                  <td className="px-4 py-2 max-w-[220px] truncate" title={e.target}>
                    {e.target}
                  </td>
                  <td className="px-4 py-2 text-xs text-muted-foreground">
                    {e.before || e.after ? (
                      <span>
                        {e.before ?? "—"} → {e.after ?? "—"}
                      </span>
                    ) : (
                      "—"
                    )}
                  </td>
                  <td className="px-4 py-2 text-xs">{e.source}</td>
                  <td className="px-4 py-2 text-xs text-muted-foreground whitespace-nowrap">
                    {new Date(e.timestamp).toLocaleString()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

function ResultBadge({ result }: { result: AuditResult }) {
  const config: Record<AuditResult, { label: string; className: string; Icon: typeof ShieldCheck }> = {
    SUCCESS: { label: "Success", className: "text-severity-low bg-severity-low/10", Icon: ShieldCheck },
    FAILURE: { label: "Failure", className: "text-severity-high bg-severity-high/10", Icon: ShieldX },
    DENIED: { label: "Denied", className: "text-severity-medium bg-severity-medium/10", Icon: ShieldAlert },
  };
  const { label, className, Icon } = config[result];
  return (
    <span className={cn("inline-flex items-center gap-1 text-xs px-2 py-0.5 rounded-full", className)}>
      <Icon className="h-3 w-3" />
      {label}
    </span>
  );
}
