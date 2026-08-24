import { useEffect, useState } from "react";
import { Cpu, MemoryStick, HardDrive, Clock, Server } from "lucide-react";
import { getSystemMetrics, onSystemMetrics } from "@/lib/ipc";
import { formatBytes, formatUptime } from "@/lib/utils";
import type { SystemMetrics } from "@/types";

export default function HealthPage() {
  const [metrics, setMetrics] = useState<SystemMetrics | null>(null);

  useEffect(() => {
    getSystemMetrics().then(setMetrics);
    const unlisten = onSystemMetrics(setMetrics);
    return () => {
      unlisten.then((u) => u());
    };
  }, []);

  const ramPercent = metrics
    ? Math.round((metrics.ram_used_bytes / metrics.ram_total_bytes) * 100)
    : 0;

  return (
    <div className="p-6 space-y-4">
      <div>
        <h1 className="text-xl font-semibold">Health</h1>
        <p className="text-sm text-muted-foreground">
          {metrics?.host_name ?? "—"} · {metrics?.os_version ?? "—"}
        </p>
      </div>

      <div className="grid md:grid-cols-2 gap-4">
        <div className="rounded-lg border border-border bg-card p-4">
          <div className="flex items-center gap-2 mb-3">
            <Cpu className="h-4 w-4 text-muted-foreground" />
            <h2 className="text-sm font-medium">CPU</h2>
          </div>
          <div className="text-3xl font-semibold tabular-nums mb-2">
            {metrics ? `${metrics.cpu_usage_percent.toFixed(0)}%` : "—"}
          </div>
          <Bar percent={metrics?.cpu_usage_percent ?? 0} />
        </div>

        <div className="rounded-lg border border-border bg-card p-4">
          <div className="flex items-center gap-2 mb-3">
            <MemoryStick className="h-4 w-4 text-muted-foreground" />
            <h2 className="text-sm font-medium">Memory</h2>
          </div>
          <div className="text-3xl font-semibold tabular-nums mb-1">{ramPercent}%</div>
          <p className="text-xs text-muted-foreground mb-2">
            {metrics ? `${formatBytes(metrics.ram_used_bytes)} / ${formatBytes(metrics.ram_total_bytes)}` : "—"}
          </p>
          <Bar percent={ramPercent} />
        </div>

        <div className="rounded-lg border border-border bg-card p-4 md:col-span-2">
          <div className="flex items-center gap-2 mb-3">
            <HardDrive className="h-4 w-4 text-muted-foreground" />
            <h2 className="text-sm font-medium">Disks</h2>
          </div>
          {!metrics || metrics.disks.length === 0 ? (
            <p className="text-sm text-muted-foreground">No disk data available.</p>
          ) : (
            <div className="space-y-3">
              {metrics.disks.map((d) => {
                const pct = Math.round((d.used_bytes / d.total_bytes) * 100) || 0;
                return (
                  <div key={d.mount_point}>
                    <div className="flex justify-between text-xs mb-1">
                      <span className="font-mono">{d.mount_point}</span>
                      <span className="text-muted-foreground">
                        {formatBytes(d.used_bytes)} / {formatBytes(d.total_bytes)} ({pct}%)
                      </span>
                    </div>
                    <Bar percent={pct} />
                  </div>
                );
              })}
            </div>
          )}
        </div>

        <div className="rounded-lg border border-border bg-card p-4">
          <div className="flex items-center gap-2 mb-3">
            <Clock className="h-4 w-4 text-muted-foreground" />
            <h2 className="text-sm font-medium">Uptime</h2>
          </div>
          <div className="text-3xl font-semibold">
            {metrics ? formatUptime(metrics.uptime_seconds) : "—"}
          </div>
        </div>

        <div className="rounded-lg border border-border bg-card p-4">
          <div className="flex items-center gap-2 mb-3">
            <Server className="h-4 w-4 text-muted-foreground" />
            <h2 className="text-sm font-medium">Processes</h2>
          </div>
          <div className="text-3xl font-semibold tabular-nums">
            {metrics ? metrics.process_count : "—"}
          </div>
        </div>
      </div>

      <p className="text-xs text-muted-foreground rounded-md border border-border bg-card px-3 py-2">
        Battery, temperature, and storage-health details aren't shown — Windows doesn't expose
        those reliably through the APIs used here on all hardware, and this page doesn't invent
        numbers when it can't get real ones.
      </p>
    </div>
  );
}

function Bar({ percent }: { percent: number }) {
  return (
    <div className="h-1.5 rounded-full bg-muted overflow-hidden">
      <div
        className="h-full bg-primary transition-all"
        style={{ width: `${Math.min(100, Math.max(0, percent))}%` }}
      />
    </div>
  );
}
