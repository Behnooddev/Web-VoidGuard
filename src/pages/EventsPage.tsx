import { useEffect, useState } from "react";
import { RefreshCw, Play, AlertTriangle } from "lucide-react";
import { getRecentEvents, getRecentRiskFindings, runRiskAnalysis } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import EmptyState from "@/components/EmptyState";
import type { SystemEvent, EventCategory } from "@/types";
import type { RiskFinding } from "@/types/monitoring";

const FILTERS: { label: string; categories: EventCategory[] | null }[] = [
  { label: "All", categories: null },
  {
    label: "Security",
    categories: ["SECURITY_SETTING_CHANGED", "FIREWALL_CHANGED", "STARTUP_CHANGED"],
  },
  { label: "Network", categories: ["PORT_OPENED", "PORT_CLOSED", "NETWORK_CHANGED", "DNS_CHANGED"] },
  { label: "Files", categories: ["FILE_CREATED", "FILE_MODIFIED", "FILE_DELETED"] },
  { label: "Processes", categories: ["PROCESS_STARTED", "PROCESS_STOPPED"] },
  { label: "Services", categories: ["SERVICE_CHANGED"] },
];

export default function EventsPage() {
  const [events, setEvents] = useState<SystemEvent[]>([]);
  const [findings, setFindings] = useState<RiskFinding[]>([]);
  const [filter, setFilter] = useState(FILTERS[0]);
  const [loading, setLoading] = useState(true);
  const [analyzing, setAnalyzing] = useState(false);

  const refresh = () => {
    setLoading(true);
    Promise.all([getRecentEvents(100), getRecentRiskFindings(20)])
      .then(([e, f]) => {
        setEvents(e);
        setFindings(f);
      })
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 6000);
    return () => clearInterval(interval);
  }, []);

  const handleAnalyze = async () => {
    setAnalyzing(true);
    try {
      await runRiskAnalysis();
      refresh();
    } finally {
      setAnalyzing(false);
    }
  };

  const filtered = filter.categories
    ? events.filter((e) => filter.categories!.includes(e.category))
    : events;

  return (
    <div className="p-6 space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold">Events</h1>
          <p className="text-sm text-muted-foreground">Timeline and correlated risk findings</p>
        </div>
        <div className="flex gap-2">
          <button
            onClick={handleAnalyze}
            disabled={analyzing}
            className="flex items-center gap-2 text-sm px-3 py-1.5 rounded-md border border-border hover:bg-muted transition-colors disabled:opacity-50"
          >
            <Play className={cn("h-3.5 w-3.5", analyzing && "animate-pulse")} />
            Run risk analysis
          </button>
          <button
            onClick={refresh}
            className="flex items-center gap-2 text-sm px-3 py-1.5 rounded-md border border-border hover:bg-muted transition-colors"
          >
            <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
            Refresh
          </button>
        </div>
      </div>

      {findings.length > 0 && (
        <div className="rounded-lg border border-severity-medium/40 bg-severity-medium/5 overflow-hidden">
          <div className="px-4 py-3 border-b border-severity-medium/30 flex items-center gap-2">
            <AlertTriangle className="h-4 w-4 text-severity-medium" />
            <h2 className="text-sm font-medium">Risk findings</h2>
          </div>
          <ul className="divide-y divide-border">
            {findings.map((f) => (
              <li key={f.id} className="px-4 py-3">
                <div className="flex items-center justify-between mb-1">
                  <p className="text-sm font-medium">{f.title}</p>
                  <div className="flex gap-1.5">
                    <SeverityBadge severity={f.severity} />
                    <span className="text-xs px-2 py-0.5 rounded-full bg-muted text-muted-foreground">
                      {f.confidence} confidence
                    </span>
                  </div>
                </div>
                <p className="text-xs text-muted-foreground mb-1.5">{f.description}</p>
                <ul className="text-xs text-muted-foreground list-disc list-inside space-y-0.5 mb-1.5">
                  {f.evidence.map((ev, i) => (
                    <li key={i}>{ev}</li>
                  ))}
                </ul>
                {f.remediation && (
                  <p className="text-xs text-primary">{f.remediation}</p>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}

      <div className="flex gap-1 flex-wrap">
        {FILTERS.map((f) => (
          <button
            key={f.label}
            onClick={() => setFilter(f)}
            className={cn(
              "text-xs px-3 py-1.5 rounded-full border transition-colors",
              filter.label === f.label
                ? "border-primary text-primary bg-accent"
                : "border-border text-muted-foreground hover:bg-muted"
            )}
          >
            {f.label}
          </button>
        ))}
      </div>

      <div className="rounded-lg border border-border bg-card">
        {filtered.length === 0 ? (
          <EmptyState title="No events" description="Nothing matches this filter yet." />
        ) : (
          <ul className="divide-y divide-border">
            {filtered.map((e) => (
              <li key={e.id} className="px-4 py-3 flex items-start gap-3 text-sm">
                <span
                  className="mt-1 h-2 w-2 rounded-full shrink-0"
                  style={{ background: severityColor(e.severity) }}
                />
                <div className="flex-1 min-w-0">
                  <p className="font-medium">{e.description}</p>
                  <p className="text-xs text-muted-foreground">
                    {new Date(e.timestamp).toLocaleString()} · {e.source} · {e.category}
                  </p>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

function severityColor(sev: SystemEvent["severity"]): string {
  switch (sev) {
    case "CRITICAL":
      return "hsl(var(--severity-critical))";
    case "HIGH":
      return "hsl(var(--severity-high))";
    case "MEDIUM":
      return "hsl(var(--severity-medium))";
    case "LOW":
      return "hsl(var(--severity-low))";
    default:
      return "hsl(var(--severity-info))";
  }
}

function SeverityBadge({ severity }: { severity: RiskFinding["severity"] }) {
  const styles: Record<RiskFinding["severity"], string> = {
    INFO: "text-severity-info bg-severity-info/10",
    LOW: "text-severity-low bg-severity-low/10",
    MEDIUM: "text-severity-medium bg-severity-medium/10",
    HIGH: "text-severity-high bg-severity-high/10",
    CRITICAL: "text-severity-critical bg-severity-critical/10",
  };
  return (
    <span className={cn("text-xs px-2 py-0.5 rounded-full", styles[severity])}>{severity}</span>
  );
}
