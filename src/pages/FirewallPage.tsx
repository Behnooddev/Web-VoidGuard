import { useEffect, useState } from "react";
import { Plus, RefreshCw, Trash2, Power } from "lucide-react";
import {
  createFirewallRule,
  deleteFirewallRule,
  listFirewallRules,
  setFirewallRuleEnabled,
} from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { AppError } from "@/types/process";
import type {
  CreateFirewallRuleRequest,
  FirewallAction,
  FirewallProtocol,
  FirewallRule,
} from "@/types/firewall";
import type { PortDirection } from "@/types/ports";
import EmptyState from "@/components/EmptyState";

type Pending =
  | { kind: "delete"; rule: FirewallRule }
  | { kind: "toggle"; rule: FirewallRule };

const EMPTY_FORM: CreateFirewallRuleRequest = {
  name: "",
  description: null,
  protocol: "TCP",
  direction: "INBOUND",
  action: "BLOCK",
  local_ports: null,
  remote_ports: null,
  remote_addresses: null,
  application_path: null,
  enabled: true,
};

export default function FirewallPage() {
  const [rules, setRules] = useState<FirewallRule[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<AppError | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [pending, setPending] = useState<Pending | null>(null);
  const [actionError, setActionError] = useState<AppError | null>(null);

  const refresh = () => {
    setLoading(true);
    listFirewallRules()
      .then((r) => {
        setRules(r);
        setLoadError(null);
      })
      .catch((e: AppError) => setLoadError(e))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    refresh();
  }, []);

  const runPending = async () => {
    if (!pending) return;
    setActionError(null);
    try {
      if (pending.kind === "delete") {
        await deleteFirewallRule(pending.rule.name);
      } else {
        await setFirewallRuleEnabled({
          name: pending.rule.name,
          enabled: !pending.rule.enabled,
        });
      }
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
          <h1 className="text-xl font-semibold">Firewall</h1>
          <p className="text-sm text-muted-foreground">
            {rules.length} rule{rules.length === 1 ? "" : "s"} created by VoidGuard
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={refresh}
            className="flex items-center gap-2 text-sm px-3 py-1.5 rounded-md border border-border hover:bg-muted transition-colors"
          >
            <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
            Refresh
          </button>
          <button
            onClick={() => setShowCreate(true)}
            className="flex items-center gap-2 text-sm px-3 py-1.5 rounded-md bg-primary text-primary-foreground hover:opacity-90 transition-opacity"
          >
            <Plus className="h-3.5 w-3.5" />
            New Rule
          </button>
        </div>
      </div>

      <p className="text-xs text-muted-foreground rounded-md border border-border bg-card px-3 py-2">
        This list shows only rules VoidGuard itself created (including quick
        allow/block actions from the Network page) — not the hundreds of
        built-in Windows and app rules already on your system.
      </p>

      {loadError && (
        <div className="rounded-md border border-severity-medium/40 bg-severity-medium/10 px-4 py-3 text-sm text-severity-medium">
          <p className="font-medium">{loadError.message}</p>
          {loadError.details && <p className="text-xs mt-1">{loadError.details}</p>}
        </div>
      )}

      <div className="rounded-lg border border-border bg-card overflow-hidden">
        {rules.length === 0 && !loading && !loadError ? (
          <EmptyState
            title="No VoidGuard-managed rules yet"
            description="Create one, or allow/block a port from the Network page."
          />
        ) : (
          <table className="w-full text-sm">
            <thead className="bg-muted text-muted-foreground text-xs uppercase tracking-wide">
              <tr>
                <th className="text-left px-4 py-2 font-medium">Name</th>
                <th className="text-left px-4 py-2 font-medium">Action</th>
                <th className="text-left px-4 py-2 font-medium">Direction</th>
                <th className="text-left px-4 py-2 font-medium">Protocol</th>
                <th className="text-left px-4 py-2 font-medium">Local Ports</th>
                <th className="text-left px-4 py-2 font-medium">Remote</th>
                <th className="text-left px-4 py-2 font-medium">Enabled</th>
                <th className="px-4 py-2 text-right font-medium">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              {rules.map((r) => (
                <tr key={r.name} className="hover:bg-muted/50 align-top">
                  <td className="px-4 py-2 font-medium">
                    {r.name}
                    {r.description && (
                      <p className="text-xs text-muted-foreground font-normal">
                        {r.description}
                      </p>
                    )}
                  </td>
                  <td className="px-4 py-2">
                    <ActionBadge action={r.action} />
                  </td>
                  <td className="px-4 py-2 text-xs">{r.direction}</td>
                  <td className="px-4 py-2 font-mono text-xs">{r.protocol}</td>
                  <td className="px-4 py-2 font-mono text-xs">{r.local_ports ?? "Any"}</td>
                  <td className="px-4 py-2 font-mono text-xs">
                    {r.remote_addresses ?? "Any"}
                  </td>
                  <td className="px-4 py-2">
                    <span
                      className={cn(
                        "text-xs px-2 py-0.5 rounded-full",
                        r.enabled
                          ? "text-severity-low bg-severity-low/10"
                          : "text-muted-foreground bg-muted"
                      )}
                    >
                      {r.enabled ? "Enabled" : "Disabled"}
                    </span>
                  </td>
                  <td className="px-4 py-2">
                    <div className="flex justify-end gap-1">
                      <button
                        onClick={() => setPending({ kind: "toggle", rule: r })}
                        className="text-muted-foreground hover:bg-muted rounded-md p-1.5 transition-colors"
                        title={r.enabled ? "Disable rule" : "Enable rule"}
                      >
                        <Power className="h-4 w-4" />
                      </button>
                      <button
                        onClick={() => setPending({ kind: "delete", rule: r })}
                        className="text-severity-high hover:bg-severity-high/10 rounded-md p-1.5 transition-colors"
                        title="Delete rule"
                      >
                        <Trash2 className="h-4 w-4" />
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {showCreate && (
        <CreateRuleDialog
          onCancel={() => setShowCreate(false)}
          onCreated={() => {
            setShowCreate(false);
            refresh();
          }}
        />
      )}

      {pending && (
        <ConfirmDialog
          pending={pending}
          error={actionError}
          onCancel={() => {
            setPending(null);
            setActionError(null);
          }}
          onConfirm={runPending}
        />
      )}
    </div>
  );
}

function ActionBadge({ action }: { action: FirewallAction }) {
  return (
    <span
      className={cn(
        "text-xs px-2 py-0.5 rounded-full font-medium",
        action === "ALLOW"
          ? "text-severity-low bg-severity-low/10"
          : "text-severity-high bg-severity-high/10"
      )}
    >
      {action}
    </span>
  );
}

function CreateRuleDialog({
  onCancel,
  onCreated,
}: {
  onCancel: () => void;
  onCreated: () => void;
}) {
  const [form, setForm] = useState<CreateFirewallRuleRequest>(EMPTY_FORM);
  const [error, setError] = useState<AppError | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const set = <K extends keyof CreateFirewallRuleRequest>(
    key: K,
    value: CreateFirewallRuleRequest[K]
  ) => setForm((f) => ({ ...f, [key]: value }));

  const emptyToNull = (v: string) => (v.trim() === "" ? null : v.trim());

  const submit = async () => {
    if (!form.name.trim()) {
      setError({
        code: "NAME_REQUIRED",
        message: "Rule name is required.",
        details: null,
        recoverable: false,
      });
      return;
    }
    setSubmitting(true);
    setError(null);
    try {
      await createFirewallRule(form);
      onCreated();
    } catch (e) {
      setError(e as AppError);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-50" onClick={onCancel}>
      <div
        className="bg-card border border-border rounded-lg shadow-xl w-full max-w-md p-5 max-h-[90vh] overflow-y-auto"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="font-semibold text-sm mb-3">New firewall rule</h3>

        <div className="space-y-3">
          <Field label="Name">
            <input
              value={form.name}
              onChange={(e) => set("name", e.target.value)}
              placeholder="e.g. Allow dev server"
              className="w-full px-3 py-1.5 text-sm rounded-md border border-border bg-background focus:outline-none focus:ring-2 focus:ring-primary"
            />
          </Field>

          <Field label="Description (optional)">
            <input
              value={form.description ?? ""}
              onChange={(e) => set("description", emptyToNull(e.target.value))}
              className="w-full px-3 py-1.5 text-sm rounded-md border border-border bg-background focus:outline-none focus:ring-2 focus:ring-primary"
            />
          </Field>

          <div className="grid grid-cols-3 gap-2">
            <Field label="Action">
              <Select
                value={form.action}
                onChange={(v) => set("action", v as FirewallAction)}
                options={["ALLOW", "BLOCK"]}
              />
            </Field>
            <Field label="Direction">
              <Select
                value={form.direction}
                onChange={(v) => set("direction", v as PortDirection)}
                options={["INBOUND", "OUTBOUND"]}
              />
            </Field>
            <Field label="Protocol">
              <Select
                value={form.protocol}
                onChange={(v) => set("protocol", v as FirewallProtocol)}
                options={["TCP", "UDP", "ANY"]}
              />
            </Field>
          </div>

          <Field label="Local ports (optional, e.g. 8000-8010)">
            <input
              value={form.local_ports ?? ""}
              onChange={(e) => set("local_ports", emptyToNull(e.target.value))}
              className="w-full px-3 py-1.5 text-sm font-mono rounded-md border border-border bg-background focus:outline-none focus:ring-2 focus:ring-primary"
            />
          </Field>

          <Field label="Remote addresses (optional, e.g. 10.0.0.0/24)">
            <input
              value={form.remote_addresses ?? ""}
              onChange={(e) => set("remote_addresses", emptyToNull(e.target.value))}
              className="w-full px-3 py-1.5 text-sm font-mono rounded-md border border-border bg-background focus:outline-none focus:ring-2 focus:ring-primary"
            />
          </Field>

          <Field label="Application path (optional)">
            <input
              value={form.application_path ?? ""}
              onChange={(e) => set("application_path", emptyToNull(e.target.value))}
              placeholder="C:\Path\To\app.exe"
              className="w-full px-3 py-1.5 text-sm font-mono rounded-md border border-border bg-background focus:outline-none focus:ring-2 focus:ring-primary"
            />
          </Field>

          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={form.enabled}
              onChange={(e) => set("enabled", e.target.checked)}
            />
            Enabled immediately
          </label>
        </div>

        {error && (
          <div className="rounded-md border border-severity-high/40 bg-severity-high/10 px-3 py-2 text-xs text-severity-high mt-3">
            <p className="font-medium">{error.message}</p>
            {error.details && <p className="mt-0.5">{error.details}</p>}
          </div>
        )}

        <div className="flex justify-end gap-2 mt-4">
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
            {submitting ? "Creating…" : "Create Rule"}
          </button>
        </div>
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block">
      <span className="block text-xs text-muted-foreground mb-1">{label}</span>
      {children}
    </label>
  );
}

function Select({
  value,
  onChange,
  options,
}: {
  value: string;
  onChange: (v: string) => void;
  options: string[];
}) {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="w-full px-2 py-1.5 text-sm rounded-md border border-border bg-background focus:outline-none focus:ring-2 focus:ring-primary"
    >
      {options.map((o) => (
        <option key={o} value={o}>
          {o}
        </option>
      ))}
    </select>
  );
}

function ConfirmDialog({
  pending,
  error,
  onCancel,
  onConfirm,
}: {
  pending: Pending;
  error: AppError | null;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const copy =
    pending.kind === "delete"
      ? {
          title: "Delete this rule?",
          body: `"${pending.rule.name}" will be removed from Windows Firewall. This does not affect the process or app it was scoped to.`,
          confirmLabel: "Delete",
          confirmClass: "bg-severity-high",
        }
      : {
          title: pending.rule.enabled ? "Disable this rule?" : "Enable this rule?",
          body: `"${pending.rule.name}" will be ${pending.rule.enabled ? "disabled" : "enabled"} in Windows Firewall.`,
          confirmLabel: pending.rule.enabled ? "Disable" : "Enable",
          confirmClass: "bg-primary",
        };

  return (
    <div className="fixed inset-0 bg-black/40 flex items-center justify-center z-50" onClick={onCancel}>
      <div
        className="bg-card border border-border rounded-lg shadow-xl w-full max-w-sm p-5"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="font-semibold text-sm mb-1">{copy.title}</h3>
        <p className="text-sm text-muted-foreground mb-3">{copy.body}</p>
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
              copy.confirmClass
            )}
          >
            {copy.confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
