import { useEffect, useState } from "react";
import { RefreshCw, FilePlus, FileMinus, FilePen, FolderCheck } from "lucide-react";
import { getRecentFileEvents, getWatchScopes } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import EmptyState from "@/components/EmptyState";
import type { FileEvent, WatchScope } from "@/types/monitoring";

const ICONS: Record<FileEvent["change_type"], typeof FilePlus> = {
  CREATED: FilePlus,
  MODIFIED: FilePen,
  DELETED: FileMinus,
};

export default function FilesPage() {
  const [scopes, setScopes] = useState<WatchScope[]>([]);
  const [events, setEvents] = useState<FileEvent[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = () => {
    setLoading(true);
    Promise.all([getWatchScopes(), getRecentFileEvents(50)])
      .then(([s, e]) => {
        setScopes(s);
        setEvents(e);
      })
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 8000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="p-6 space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold">Files</h1>
          <p className="text-sm text-muted-foreground">
            Integrity monitoring for security-sensitive locations
          </p>
        </div>
        <button
          onClick={refresh}
          className="flex items-center gap-2 text-sm px-3 py-1.5 rounded-md border border-border hover:bg-muted transition-colors"
        >
          <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
          Refresh
        </button>
      </div>

      <div className="rounded-lg border border-border bg-card p-4">
        <h2 className="text-sm font-medium mb-3 flex items-center gap-2">
          <FolderCheck className="h-4 w-4 text-muted-foreground" />
          Watched locations
        </h2>
        {scopes.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No watch scopes configured yet — the watcher seeds a
            conservative default set (hosts file, Startup folders) on
            first run.
          </p>
        ) : (
          <ul className="space-y-1.5">
            {scopes.map((s) => (
              <li key={s.path} className="flex items-center justify-between text-sm">
                <span className="font-mono text-xs text-muted-foreground truncate max-w-[70%]" title={s.path}>
                  {s.path}
                </span>
                <span className="text-xs text-muted-foreground">
                  {s.label}
                  {s.built_in && (
                    <span className="ml-2 px-1.5 py-0.5 rounded-full bg-muted text-[0.65rem] uppercase tracking-wide">
                      built-in
                    </span>
                  )}
                </span>
              </li>
            ))}
          </ul>
        )}
        <p className="text-xs text-muted-foreground mt-3">
          Deliberately not the whole disk — continuous full-disk
          scanning is avoided by design. Custom scopes are planned for
          a later pass.
        </p>
      </div>

      <div className="rounded-lg border border-border bg-card">
        <div className="px-4 py-3 border-b border-border">
          <h2 className="text-sm font-medium">Recent file events</h2>
        </div>
        {events.length === 0 ? (
          <EmptyState
            title="No file events yet"
            description="Changes to a watched location will appear here as they happen."
          />
        ) : (
          <ul className="divide-y divide-border">
            {events.map((e) => {
              const Icon = ICONS[e.change_type];
              return (
                <li key={e.id} className="px-4 py-3 flex items-start gap-3 text-sm">
                  <Icon className="h-4 w-4 mt-0.5 text-muted-foreground shrink-0" />
                  <div className="flex-1 min-w-0">
                    <p className="font-mono text-xs truncate" title={e.path}>
                      {e.path}
                    </p>
                    <p className="text-xs text-muted-foreground mt-0.5">
                      {e.change_type} · {new Date(e.timestamp).toLocaleString()}
                      {e.sha256 && (
                        <>
                          {" "}
                          · <span className="font-mono">{e.sha256.slice(0, 12)}…</span>
                        </>
                      )}
                    </p>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </div>
  );
}
