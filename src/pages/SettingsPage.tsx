import { useEffect, useState } from "react";
import { Bell, Palette, Trash2 } from "lucide-react";
import {
  getNotificationSettings,
  getRetentionSettings,
  runRetentionCleanup,
  setNotificationSettings,
  setRetentionSettings,
} from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { NotificationSettings, RetentionSettings } from "@/types/scan";

type Theme = "light" | "dark" | "system";
const SEVERITIES: NotificationSettings["min_severity"][] = ["LOW", "MEDIUM", "HIGH", "CRITICAL"];
const RETENTION_FIELDS: { key: keyof RetentionSettings; label: string }[] = [
  { key: "events_days", label: "Event log" },
  { key: "process_snapshots_days", label: "Process snapshots" },
  { key: "port_snapshots_days", label: "Port snapshots" },
];

export default function SettingsPage() {
  const [theme, setTheme] = useState<Theme>(
    () => (localStorage.getItem("theme") as Theme) || "system"
  );
  const [notif, setNotif] = useState<NotificationSettings | null>(null);
  const [retention, setRetention] = useState<RetentionSettings | null>(null);
  const [saving, setSaving] = useState(false);
  const [cleaning, setCleaning] = useState(false);
  const [cleanupMessage, setCleanupMessage] = useState<string | null>(null);

  useEffect(() => {
    getNotificationSettings().then(setNotif);
    getRetentionSettings().then(setRetention);
  }, []);

  const applyTheme = (t: Theme) => {
    setTheme(t);
    localStorage.setItem("theme", t);
    const isDark =
      t === "dark" || (t === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
    document.documentElement.classList.toggle("dark", isDark);
  };

  const updateNotif = async (patch: Partial<NotificationSettings>) => {
    if (!notif) return;
    const next = { ...notif, ...patch };
    setNotif(next);
    setSaving(true);
    try {
      await setNotificationSettings(next);
    } finally {
      setSaving(false);
    }
  };

  const updateRetention = async (patch: Partial<RetentionSettings>) => {
    if (!retention) return;
    const next = { ...retention, ...patch };
    setRetention(next);
    setSaving(true);
    try {
      await setRetentionSettings(next);
    } finally {
      setSaving(false);
    }
  };

  const cleanupNow = async () => {
    setCleaning(true);
    setCleanupMessage(null);
    try {
      const result = await runRetentionCleanup();
      const total =
        result.events_deleted + result.process_snapshots_deleted + result.port_snapshots_deleted;
      setCleanupMessage(
        total === 0 ? "Nothing to clean up." : `Removed ${total} old row${total === 1 ? "" : "s"}.`
      );
    } finally {
      setCleaning(false);
    }
  };

  return (
    <div className="p-6 space-y-4 max-w-2xl">
      <div>
        <h1 className="text-xl font-semibold">Settings</h1>
        <p className="text-sm text-muted-foreground">
          Appearance, notifications, and data retention.
        </p>
      </div>

      <div className="rounded-lg border border-border bg-card p-4">
        <div className="flex items-center gap-2 mb-3">
          <Palette className="h-4 w-4 text-muted-foreground" />
          <h2 className="text-sm font-medium">Appearance</h2>
        </div>
        <div className="flex gap-2">
          {(["light", "dark", "system"] as Theme[]).map((t) => (
            <button
              key={t}
              onClick={() => applyTheme(t)}
              className={cn(
                "flex-1 text-sm px-3 py-2 rounded-md border capitalize transition-colors",
                theme === t
                  ? "border-primary text-primary bg-accent"
                  : "border-border text-muted-foreground hover:bg-muted"
              )}
            >
              {t}
            </button>
          ))}
        </div>
      </div>

      <div className="rounded-lg border border-border bg-card p-4">
        <div className="flex items-center gap-2 mb-3">
          <Bell className="h-4 w-4 text-muted-foreground" />
          <h2 className="text-sm font-medium">Notifications</h2>
        </div>

        {!notif ? (
          <p className="text-sm text-muted-foreground">Loading…</p>
        ) : (
          <div className="space-y-3">
            <label className="flex items-center justify-between text-sm cursor-pointer">
              <span>Show desktop notifications for security events</span>
              <input
                type="checkbox"
                checked={notif.enabled}
                onChange={(e) => updateNotif({ enabled: e.target.checked })}
                className="h-4 w-4"
              />
            </label>

            <div>
              <p className="text-sm mb-1.5">Minimum severity</p>
              <div className="flex gap-2">
                {SEVERITIES.map((s) => (
                  <button
                    key={s}
                    disabled={!notif.enabled}
                    onClick={() => updateNotif({ min_severity: s })}
                    className={cn(
                      "text-xs px-2.5 py-1.5 rounded-full border transition-colors disabled:opacity-40",
                      notif.min_severity === s
                        ? "border-primary text-primary bg-accent"
                        : "border-border text-muted-foreground hover:bg-muted"
                    )}
                  >
                    {s}
                  </button>
                ))}
              </div>
              <p className="text-xs text-muted-foreground mt-1.5">
                Only events at or above this severity trigger a desktop notification. Everything
                still appears on the Events page regardless of this setting.
              </p>
            </div>
          </div>
        )}
      </div>

      <div className="rounded-lg border border-border bg-card p-4">
        <div className="flex items-center gap-2 mb-3">
          <Trash2 className="h-4 w-4 text-muted-foreground" />
          <h2 className="text-sm font-medium">Data retention</h2>
        </div>

        {!retention ? (
          <p className="text-sm text-muted-foreground">Loading…</p>
        ) : (
          <div className="space-y-3">
            {RETENTION_FIELDS.map(({ key, label }) => (
              <label key={key} className="flex items-center justify-between text-sm gap-3">
                <span>{label}</span>
                <span className="flex items-center gap-2">
                  <input
                    type="number"
                    min={0}
                    value={retention[key]}
                    onChange={(e) =>
                      updateRetention({ [key]: Math.max(0, Number(e.target.value)) })
                    }
                    className="w-20 px-2 py-1 text-sm rounded-md border border-border bg-background text-right focus:outline-none focus:ring-2 focus:ring-primary"
                  />
                  <span className="text-xs text-muted-foreground">days (0 = keep forever)</span>
                </span>
              </label>
            ))}

            <div className="flex items-center gap-3 pt-1">
              <button
                onClick={cleanupNow}
                disabled={cleaning}
                className="text-sm px-3 py-1.5 rounded-md border border-border hover:bg-muted transition-colors disabled:opacity-50"
              >
                {cleaning ? "Cleaning…" : "Clean up now"}
              </button>
              {cleanupMessage && (
                <span className="text-xs text-muted-foreground">{cleanupMessage}</span>
              )}
            </div>
            <p className="text-xs text-muted-foreground">
              Applied automatically once per launch, and any time you click "Clean up now".
            </p>
          </div>
        )}
      </div>

      {saving && <p className="text-xs text-muted-foreground">Saving…</p>}
    </div>
  );
}
