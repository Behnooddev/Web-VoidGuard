import { useEffect, useState } from "react";
import {
  Cpu,
  MemoryStick,
  HardDrive,
  Clock,
  ListTree,
  ShieldCheck,
} from "lucide-react";
import StatCard from "@/components/StatCard";
import EmptyState from "@/components/EmptyState";
import {
  getLatestSecurityScore,
  getRecentEvents,
  getSystemMetrics,
  onSecurityScoreUpdated,
  onSystemMetrics,
} from "@/lib/ipc";
import { formatBytes, formatUptime } from "@/lib/utils";
import type { SystemEvent, SystemMetrics } from "@/types";
import type { SecurityScore } from "@/types/scan";

export default function Dashboard() {
  const [metrics, setMetrics] = useState<SystemMetrics | null>(null);
  const [events, setEvents] = useState<SystemEvent[]>([]);
  const [score, setScore] = useState<SecurityScore | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getSystemMetrics().then(setMetrics).catch((e) => setError(String(e)));
    getRecentEvents(10).then(setEvents).catch(() => {});
    getLatestSecurityScore().then(setScore).catch(() => {});

    const unlistenMetrics = onSystemMetrics(setMetrics);
    const unlistenScore = onSecurityScoreUpdated(setScore);
    const interval = setInterval(() => {
      getRecentEvents(10).then(setEvents).catch(() => {});
    }, 5000);

    return () => {
      unlistenMetrics.then((unlisten) => unlisten());
      unlistenScore.then((unlisten) => unlisten());
      clearInterval(interval);
    };
  }, []);

  const ramPercent = metrics
    ? Math.round((metrics.ram_used_bytes / metrics.ram_total_bytes) * 100)
    : 0;
  const primaryDisk = metrics?.disks[0];
  const diskPercent = primaryDisk
    ? Math.round((primaryDisk.used_bytes / primaryDisk.total_bytes) * 100)
    : 0;

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-xl font-semibold">Dashboard</h1>
        <p className="text-sm text-muted-foreground">
          {metrics?.host_name ?? "\u2014"} \u00b7 {metrics?.os_version ?? "\u2014"}
        </p>
      </div>

      {error && (
        <div className="rounded-md border border-severity-high/40 bg-severity-high/10 px-4 py-3 text-sm text-severity-high">
          Failed to load system metrics: {error}
        </div>
      )}

      <div className="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-6 gap-4">
        <StatCard
          label="CPU"
          value={metrics ? `${metrics.cpu_usage_percent.toFixed(0)}%` : "\u2014"}
          icon={Cpu}
          tone={metrics && metrics.cpu_usage_percent > 85 ? "warn" : "default"}
        />
        <StatCard
          label="RAM"
          value={metrics ? `${ramPercent}%` : "\u2014"}
          subtext={
            metrics
              ? `${formatBytes(metrics.ram_used_bytes)} / ${formatBytes(metrics.ram_total_bytes)}`
              : undefined
          }
          icon={MemoryStick}
          tone={ramPercent > 85 ? "warn" : "default"}
        />
        <StatCard
          label="Disk"
          value={primaryDisk ? `${diskPercent}%` : "\u2014"}
          subtext={primaryDisk?.mount_point}
          icon={HardDrive}
          tone={diskPercent > 90 ? "warn" : "default"}
        />
        <StatCard
          label="Processes"
          value={metrics ? String(metrics.process_count) : "\u2014"}
          icon={ListTree}
        />
        <StatCard
          label="Uptime"
          value={metrics ? formatUptime(metrics.uptime_seconds) : "\u2014"}
          icon={Clock}
        />
        <StatCard
          label="Security Score"
          value={score ? String(score.score) : "\u2014"}
          subtext={
            score
              ? score.reasons[0]?.label ?? "No issues found"
              : "Not calculated yet"
          }
          icon={ShieldCheck}
          tone={
            !score ? "default" : score.score >= 80 ? "good" : score.score >= 50 ? "warn" : "bad"
          }
        />
      </div>

      <div className="rounded-lg border border-border bg-card">
        <div className="px-4 py-3 border-b border-border">
          <h2 className="text-sm font-medium">Recent Events</h2>
        </div>
        {events.length === 0 ? (
          <EmptyState
            title="No events yet"
            description="Activity across processes, network, firewall, startup, and files will appear here as it happens."
          />
        ) : (
          <ul className="divide-y divide-border">
            {events.map((e) => (
              <li key={e.id} className="px-4 py-3 flex items-start gap-3 text-sm">
                <span
                  className="mt-1 h-2 w-2 rounded-full shrink-0"
                  style={{ background: severityColor(e.severity) }}
                />
                <div className="flex-1 min-w-0">
                  <p className="font-medium">{e.description}</p>
                  <p className="text-xs text-muted-foreground">
                    {new Date(e.timestamp).toLocaleString()} \u00b7 {e.source}
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
