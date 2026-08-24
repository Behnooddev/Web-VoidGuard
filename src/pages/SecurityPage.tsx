import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { RefreshCw, ShieldCheck, ScanLine } from "lucide-react";
import { computeSecurityScore, getLatestSecurityScore, onSecurityScoreUpdated } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import type { SecurityScore } from "@/types/scan";

export default function SecurityPage() {
  const [score, setScore] = useState<SecurityScore | null>(null);
  const [loading, setLoading] = useState(true);
  const navigate = useNavigate();

  useEffect(() => {
    getLatestSecurityScore()
      .then(setScore)
      .finally(() => setLoading(false));
    const unlisten = onSecurityScoreUpdated(setScore);
    return () => {
      unlisten.then((u) => u());
    };
  }, []);

  const handleRecalculate = async () => {
    setLoading(true);
    try {
      const s = await computeSecurityScore();
      setScore(s);
    } finally {
      setLoading(false);
    }
  };

  const tone = score
    ? score.score >= 80
      ? "good"
      : score.score >= 50
      ? "warn"
      : "bad"
    : "default";

  return (
    <div className="p-6 space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold">Security</h1>
          <p className="text-sm text-muted-foreground">
            {score
              ? `Last calculated ${new Date(score.calculated_at).toLocaleString()}`
              : "No score calculated yet"}
          </p>
        </div>
        <button
          onClick={handleRecalculate}
          disabled={loading}
          className="flex items-center gap-2 text-sm px-3 py-1.5 rounded-md border border-border hover:bg-muted transition-colors disabled:opacity-50"
        >
          <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
          Recalculate
        </button>
      </div>

      <div className="rounded-lg border border-border bg-card p-6 flex items-center gap-6">
        <div
          className={cn(
            "h-24 w-24 rounded-full border-4 flex items-center justify-center shrink-0",
            tone === "good" && "border-severity-low",
            tone === "warn" && "border-severity-medium",
            tone === "bad" && "border-severity-high",
            tone === "default" && "border-border"
          )}
        >
          <span className="text-2xl font-semibold tabular-nums">{score?.score ?? "—"}</span>
        </div>
        <div>
          <div className="flex items-center gap-2 mb-1">
            <ShieldCheck className="h-4 w-4 text-muted-foreground" />
            <h2 className="text-sm font-medium">Security Score</h2>
          </div>
          <p className="text-sm text-muted-foreground max-w-md">
            Combines firewall status, startup entry classifications, open port risk, and recent
            correlated risk findings. Every point lost below has a reason attached — never a bare
            number.
          </p>
        </div>
      </div>

      <div className="rounded-lg border border-border bg-card">
        <div className="px-4 py-3 border-b border-border">
          <h2 className="text-sm font-medium">Why this score</h2>
        </div>
        {!score || score.reasons.length === 0 ? (
          <div className="px-4 py-8 text-center text-sm text-muted-foreground">
            No breakdown available yet — click Recalculate.
          </div>
        ) : (
          <ul className="divide-y divide-border">
            {score.reasons.map((r, i) => (
              <li key={i} className="px-4 py-3 flex items-center justify-between text-sm">
                <span>{r.label}</span>
                <span
                  className={cn(
                    "tabular-nums font-medium",
                    r.impact < 0 ? "text-severity-high" : "text-muted-foreground"
                  )}
                >
                  {r.impact > 0 ? `+${r.impact}` : r.impact}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>

      <button
        onClick={() => navigate("/scans")}
        className="flex items-center gap-2 text-sm px-3 py-2 rounded-md border border-border hover:bg-muted transition-colors"
      >
        <ScanLine className="h-4 w-4" />
        Run a scan for a deeper check
      </button>
    </div>
  );
}
