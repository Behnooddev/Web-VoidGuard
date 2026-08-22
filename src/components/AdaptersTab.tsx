import { useEffect, useState } from "react";
import { RefreshCw, Wifi, Cable, Shield, HelpCircle } from "lucide-react";
import { listNetworkAdapters } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { AppError } from "@/types/process";
import type { NetworkAdapter } from "@/types/system_control";
import EmptyState from "@/components/EmptyState";

const ICONS: Record<NetworkAdapter["adapter_type"], typeof Wifi> = {
  "Wi-Fi": Wifi,
  Ethernet: Cable,
  VPN: Shield,
  Loopback: HelpCircle,
  Other: HelpCircle,
};

export default function AdaptersTab() {
  const [adapters, setAdapters] = useState<NetworkAdapter[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<AppError | null>(null);

  const refresh = () => {
    setLoading(true);
    listNetworkAdapters()
      .then((a) => {
        setAdapters(a);
        setError(null);
      })
      .catch((e: AppError) => setError(e))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 10000);
    return () => clearInterval(interval);
  }, []);

  if (error) {
    return (
      <div className="rounded-md border border-severity-medium/40 bg-severity-medium/10 px-4 py-3 text-sm text-severity-medium">
        <p className="font-medium">{error.message}</p>
        {error.details && <p className="text-xs mt-1">{error.details}</p>}
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <div className="flex justify-end">
        <button
          onClick={refresh}
          className="flex items-center gap-2 text-sm px-3 py-1.5 rounded-md border border-border hover:bg-muted transition-colors"
        >
          <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
          Refresh
        </button>
      </div>

      {adapters.length === 0 && !loading ? (
        <div className="rounded-lg border border-border bg-card">
          <EmptyState title="No adapters found" />
        </div>
      ) : (
        <div className="grid md:grid-cols-2 gap-4">
          {adapters.map((a) => {
            const Icon = ICONS[a.adapter_type];
            return (
              <div key={a.name} className="rounded-lg border border-border bg-card p-4">
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-2">
                    <Icon className="h-4 w-4 text-muted-foreground" />
                    <span className="font-medium text-sm">{a.name}</span>
                  </div>
                  <span
                    className={cn(
                      "text-xs px-2 py-0.5 rounded-full",
                      a.status === "Up"
                        ? "text-severity-low bg-severity-low/10"
                        : "text-muted-foreground bg-muted"
                    )}
                  >
                    {a.status}
                  </span>
                </div>
                <p className="text-xs text-muted-foreground mb-3">{a.description}</p>
                <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs">
                  <dt className="text-muted-foreground">IPv4</dt>
                  <dd className="font-mono">{a.ipv4_addresses.join(", ") || "\u2014"}</dd>
                  <dt className="text-muted-foreground">IPv6</dt>
                  <dd className="font-mono truncate">{a.ipv6_addresses.join(", ") || "\u2014"}</dd>
                  <dt className="text-muted-foreground">Gateway</dt>
                  <dd className="font-mono">{a.gateway ?? "\u2014"}</dd>
                  <dt className="text-muted-foreground">DNS</dt>
                  <dd className="font-mono">{a.dns_servers.join(", ") || "\u2014"}</dd>
                  <dt className="text-muted-foreground">MAC</dt>
                  <dd className="font-mono">{a.mac_address ?? "\u2014"}</dd>
                  <dt className="text-muted-foreground">DHCP</dt>
                  <dd>{a.dhcp_enabled == null ? "\u2014" : a.dhcp_enabled ? "Enabled" : "Disabled"}</dd>
                  <dt className="text-muted-foreground">Link speed</dt>
                  <dd>{a.link_speed_mbps ? `${a.link_speed_mbps} Mbps` : "\u2014"}</dd>
                </dl>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
