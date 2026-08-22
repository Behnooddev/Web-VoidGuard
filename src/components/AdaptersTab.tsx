import { useEffect, useState } from "react";
import { RefreshCw, Wifi, Cable, Shield, HelpCircle, Pencil } from "lucide-react";
import { changeDns, listNetworkAdapters } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { AppError } from "@/types/process";
import type { DnsMode, NetworkAdapter } from "@/types/system_control";
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
  const [editing, setEditing] = useState<NetworkAdapter | null>(null);

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
                  <dd className="font-mono flex items-center gap-1.5">
                    <span className="truncate">{a.dns_servers.join(", ") || "\u2014"}</span>
                    <button
                      onClick={() => setEditing(a)}
                      className="shrink-0 text-muted-foreground hover:text-foreground hover:bg-muted rounded p-0.5 transition-colors"
                      title="Edit DNS"
                    >
                      <Pencil className="h-3 w-3" />
                    </button>
                  </dd>
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

      {editing && (
        <DnsDialog
          adapter={editing}
          onCancel={() => setEditing(null)}
          onSaved={() => {
            setEditing(null);
            refresh();
          }}
        />
      )}
    </div>
  );
}

function DnsDialog({
  adapter,
  onCancel,
  onSaved,
}: {
  adapter: NetworkAdapter;
  onCancel: () => void;
  onSaved: () => void;
}) {
  const [mode, setMode] = useState<DnsMode>(adapter.dhcp_enabled ? "DHCP" : "STATIC");
  const [primary, setPrimary] = useState(adapter.dns_servers[0] ?? "");
  const [secondary, setSecondary] = useState(adapter.dns_servers[1] ?? "");
  const [error, setError] = useState<AppError | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const submit = async () => {
    setError(null);
    setSubmitting(true);
    try {
      await changeDns({
        adapter_id: adapter.adapter_id,
        mode,
        primary_dns: mode === "STATIC" ? primary.trim() || null : null,
        secondary_dns: mode === "STATIC" ? secondary.trim() || null : null,
      });
      onSaved();
    } catch (e) {
      setError(e as AppError);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-50" onClick={onCancel}>
      <div
        className="bg-card border border-border rounded-lg shadow-xl w-full max-w-sm p-5"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="font-semibold text-sm mb-1">Edit DNS — {adapter.name}</h3>
        <p className="text-sm text-muted-foreground mb-3">
          Applies immediately through Windows' native DNS API. No shell commands
          are used.
        </p>

        <div className="flex gap-1 mb-3 border-b border-border">
          {(["DHCP", "STATIC"] as DnsMode[]).map((m) => (
            <button
              key={m}
              onClick={() => setMode(m)}
              className={cn(
                "px-3 py-2 text-sm border-b-2 -mb-px transition-colors",
                mode === m
                  ? "border-primary text-primary font-medium"
                  : "border-transparent text-muted-foreground hover:text-foreground"
              )}
            >
              {m === "DHCP" ? "Automatic (DHCP)" : "Static"}
            </button>
          ))}
        </div>

        {mode === "STATIC" && (
          <div className="space-y-2 mb-3">
            <label className="block">
              <span className="block text-xs text-muted-foreground mb-1">
                Primary DNS server
              </span>
              <input
                value={primary}
                onChange={(e) => setPrimary(e.target.value)}
                placeholder="e.g. 1.1.1.1"
                className="w-full px-3 py-1.5 text-sm font-mono rounded-md border border-border bg-background focus:outline-none focus:ring-2 focus:ring-primary"
              />
            </label>
            <label className="block">
              <span className="block text-xs text-muted-foreground mb-1">
                Secondary DNS server (optional)
              </span>
              <input
                value={secondary}
                onChange={(e) => setSecondary(e.target.value)}
                placeholder="e.g. 1.0.0.1"
                className="w-full px-3 py-1.5 text-sm font-mono rounded-md border border-border bg-background focus:outline-none focus:ring-2 focus:ring-primary"
              />
            </label>
          </div>
        )}

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
            onClick={submit}
            disabled={submitting}
            className="px-3 py-1.5 text-sm rounded-md bg-primary text-primary-foreground hover:opacity-90 transition-opacity disabled:opacity-50"
          >
            {submitting ? "Applying…" : "Apply"}
          </button>
        </div>
      </div>
    </div>
  );
}
