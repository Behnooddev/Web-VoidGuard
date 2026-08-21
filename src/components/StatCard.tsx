import { type LucideIcon } from "lucide-react";
import { cn } from "@/lib/utils";

interface StatCardProps {
  label: string;
  value: string;
  subtext?: string;
  icon: LucideIcon;
  tone?: "default" | "good" | "warn" | "bad";
}

const TONE_CLASSES: Record<NonNullable<StatCardProps["tone"]>, string> = {
  default: "text-foreground",
  good: "text-severity-low",
  warn: "text-severity-medium",
  bad: "text-severity-high",
};

export default function StatCard({
  label,
  value,
  subtext,
  icon: Icon,
  tone = "default",
}: StatCardProps) {
  return (
    <div className="rounded-lg border border-border bg-card p-4 flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
          {label}
        </span>
        <Icon className={cn("h-4 w-4", TONE_CLASSES[tone])} />
      </div>
      <div className={cn("text-2xl font-semibold tabular-nums", TONE_CLASSES[tone])}>
        {value}
      </div>
      {subtext && <span className="text-xs text-muted-foreground">{subtext}</span>}
    </div>
  );
}
