import { useEffect, useState } from "react";
import { Bell, Palette } from "lucide-react";
import { getNotificationSettings, setNotificationSettings } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { NotificationSettings } from "@/types/scan";

type Theme = "light" | "dark" | "system";
const SEVERITIES: NotificationSettings["min_severity"][] = ["LOW", "MEDIUM", "HIGH", "CRITICAL"];

export default function SettingsPage() {
  const [theme, setTheme] = useState<Theme>(
    () => (localStorage.getItem("theme") as Theme) || "system"
  );
  const [notif, setNotif] = useState<NotificationSettings | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    getNotificationSettings().then(setNotif);
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

  return (
    <div className="p-6 space-y-4 max-w-2xl">
      <div>
        <h1 className="text-xl font-semibold">Settings</h1>
        <p className="text-sm text-muted-foreground">
          Appearance and notifications. Full configuration management (retention, watch scopes,
          scan scheduling) is planned for a later pass.
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
        {saving && <p className="text-xs text-muted-foreground mt-2">Saving…</p>}
      </div>
    </div>
  );
}
