import { useEffect, useRef } from "react";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/api/notification";
import { getNotificationSettings, getRecentEvents } from "@/lib/ipc";
import type { SystemEvent } from "@/types";

const SEVERITY_RANK: Record<SystemEvent["severity"], number> = {
  INFO: 0,
  LOW: 1,
  MEDIUM: 2,
  HIGH: 3,
  CRITICAL: 4,
};

/**
 * Mounted once in App.tsx. Polls recent events, and for anything new
 * that meets the stored severity threshold, raises a real OS
 * notification via Tauri's Notification API. Preferences live in the
 * backend (`commands::notifications`) so Settings and this component
 * never disagree about what "enabled" means. This is the only place
 * in the app that calls the OS notification API — see
 * ARCHITECTURE.md's note on why dispatch lives client-side.
 */
export default function NotificationManager() {
  const lastSeenRef = useRef<string | null>(null);
  const permissionRef = useRef<boolean | null>(null);

  useEffect(() => {
    let cancelled = false;

    const ensurePermission = async () => {
      if (permissionRef.current !== null) return permissionRef.current;
      let granted = await isPermissionGranted();
      if (!granted) {
        const result = await requestPermission();
        granted = result === "granted";
      }
      permissionRef.current = granted;
      return granted;
    };

    const poll = async () => {
      try {
        const settings = await getNotificationSettings();
        if (!settings.enabled) return;

        const events = await getRecentEvents(20);
        if (events.length === 0) return;

        // First poll: just establish the baseline, don't notify for
        // history that predates the app opening.
        if (lastSeenRef.current === null) {
          lastSeenRef.current = events[0].timestamp;
          return;
        }

        const threshold = SEVERITY_RANK[settings.min_severity];
        const newOnes = events
          .filter((e) => e.timestamp > lastSeenRef.current!)
          .filter((e) => SEVERITY_RANK[e.severity] >= threshold)
          .reverse(); // oldest-first so notifications appear in order

        if (newOnes.length > 0) {
          const granted = await ensurePermission();
          if (granted && !cancelled) {
            for (const e of newOnes) {
              sendNotification({ title: `VoidGuard — ${e.severity}`, body: e.description });
            }
          }
        }

        lastSeenRef.current = events[0].timestamp;
      } catch {
        // Notification plumbing failing is never worth surfacing as
        // an error state to the user — just skip this tick.
      }
    };

    poll();
    const interval = setInterval(poll, 8000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  return null;
}
