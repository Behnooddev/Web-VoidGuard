import { useEffect, useState } from "react";
import { Play, Loader2, ChevronDown, ChevronRight } from "lucide-react";
import { getRecentScans, onScanProgress, runScan } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import EmptyState from "@/components/EmptyState";
import { CUSTOM_SCAN_STEP_OPTIONS } from "@/types/scan";
import type { ScanProgress, ScanResult, ScanType } from "@/types/scan";

const SCAN_TYPES: { type: ScanType; label: string; description: string }[] = [
  { type: "QUICK", label: "Quick Scan", description: "Ports, firewall, startup" },
  { type: "SYSTEM", label: "System Scan", description: "Processes, services, startup, firewall" },
  { type: "NETWORK", label: "Network Scan", description: "Adapters and open ports" },
  { type: "STARTUP", label: "Startup Scan", description: "Startup entries only" },
  { type: "INTEGRITY", label: "Integrity Scan", description: "Recent file events" },
  { type: "CUSTOM", label: "Custom Scan", description: "Pick exactly what to check" },
];

export default function ScansPage() {
  const [running, setRunning] = useState<ScanType | null>(null);
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [history, setHistory] = useState<ScanResult[]>([]);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [customSteps, setCustomSteps] = useState<string[]>([]);

  const refresh = () => {
    getRecentScans(20).then(setHistory);
  };

  useEffect(() => {
    refresh();
    const unlisten = onScanProgress(setProgress);
    return () => {
      unlisten.then((u) => u());
    };
  }, []);

  const handleRun = async (type: ScanType) => {
    setRunning(type);
    setProgress(null);
    try {
      await runScan(type, type === "CUSTOM" ? customSteps : undefined);
      refresh();
    } finally {
      setRunning(null);
      setProgress(null);
    }
  };

  return (
    <div className="p-6 space-y-4">
      <div>
        <h1 className="text-xl font-semibold">Scans</h1>
        <p className="text-sm text-muted-foreground">
          Each scan does real enumeration work — progress reflects actual steps, not a fake bar.
        </p>
      </div>

      <div className="grid md:grid-cols-2 gap-3">
        {SCAN_TYPES.map((s) => (
          <div key={s.type} className="rounded-lg border border-border bg-card p-4">
            <div className="flex items-start justify-between mb-2">
              <div>
                <h3 className="text-sm font-medium">{s.label}</h3>
                <p className="text-xs text-muted-foreground">{s.description}</p>
              </div>
              <button
                onClick={() => handleRun(s.type)}
                disabled={running !== null}
                className="flex items-center gap-1.5 text-xs px-2.5 py-1.5 rounded-md border border-border hover:bg-muted transition-colors disabled:opacity-40 disabled:pointer-events-none shrink-0"
              >
                {running === s.type ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Play className="h-3.5 w-3.5" />
                )}
                Run
              </button>
            </div>

            {s.type === "CUSTOM" && (
              <div className="flex flex-wrap gap-1.5 mt-2">
                {CUSTOM_SCAN_STEP_OPTIONS.map((opt) => (
                  <label
                    key={opt.key}
                    className={cn(
                      "text-xs px-2 py-1 rounded-full border cursor-pointer transition-colors",
                      customSteps.includes(opt.key)
                        ? "border-primary text-primary bg-accent"
                        : "border-border text-muted-foreground hover:bg-muted"
                    )}
                  >
                    <input
                      type="checkbox"
                      className="hidden"
                      checked={customSteps.includes(opt.key)}
                      onChange={(e) =>
                        setCustomSteps((prev) =>
                          e.target.checked
                            ? [...prev, opt.key]
                            : prev.filter((k) => k !== opt.key)
                        )
                      }
                    />
                    {opt.label}
                  </label>
                ))}
              </div>
            )}

            {running === s.type && progress && (
              <div className="mt-3">
                <div className="h-1.5 rounded-full bg-muted overflow-hidden">
                  <div
                    className="h-full bg-primary transition-all"
                    style={{ width: `${(progress.step_index / progress.total_steps) * 100}%` }}
                  />
                </div>
                <p className="text-xs text-muted-foreground mt-1">
                  {progress.step_label} ({progress.step_index}/{progress.total_steps}) —{" "}
                  {progress.findings_so_far} finding(s) so far
                </p>
              </div>
            )}
          </div>
        ))}
      </div>

      <div className="rounded-lg border border-border bg-card">
        <div className="px-4 py-3 border-b border-border">
          <h2 className="text-sm font-medium">Scan history</h2>
        </div>
        {history.length === 0 ? (
          <EmptyState title="No scans yet" description="Run one above to see results here." />
        ) : (
          <ul className="divide-y divide-border">
            {history.map((h) => (
              <li key={h.id}>
                <button
                  onClick={() => setExpanded(expanded === h.id ? null : h.id)}
                  className="w-full flex items-center justify-between px-4 py-3 text-sm hover:bg-muted/50 transition-colors text-left"
                >
                  <div className="flex items-center gap-2">
                    {expanded === h.id ? (
                      <ChevronDown className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                    ) : (
                      <ChevronRight className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                    )}
                    <div>
                      <span className="font-medium">{h.scan_type}</span>
                      <span className="text-muted-foreground ml-2 text-xs">
                        {new Date(h.started_at).toLocaleString()}
                      </span>
                    </div>
                  </div>
                  <span className="text-xs text-muted-foreground">{h.summary}</span>
                </button>
                {expanded === h.id && h.findings.length > 0 && (
                  <ul className="px-4 pb-3 space-y-1.5">
                    {h.findings.map((f, i) => (
                      <li key={i} className="text-xs pl-6 border-l-2 border-border ml-1.5">
                        <span
                          className={cn(
                            "font-medium",
                            (f.severity === "HIGH" || f.severity === "CRITICAL") && "text-severity-high",
                            f.severity === "MEDIUM" && "text-severity-medium",
                            (f.severity === "LOW" || f.severity === "INFO") && "text-muted-foreground"
                          )}
                        >
                          {f.label}
                        </span>
                        <p className="text-muted-foreground">{f.detail}</p>
                      </li>
                    ))}
                  </ul>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
