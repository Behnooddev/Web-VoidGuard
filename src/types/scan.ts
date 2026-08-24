export type ScanType = "QUICK" | "SYSTEM" | "NETWORK" | "STARTUP" | "INTEGRITY" | "CUSTOM";
export type ScanStatus = "RUNNING" | "COMPLETED" | "FAILED";

export interface ScanFinding {
  label: string;
  detail: string;
  severity: "INFO" | "LOW" | "MEDIUM" | "HIGH" | "CRITICAL";
}

export interface ScanResult {
  id: string;
  scan_type: ScanType;
  started_at: string;
  finished_at: string | null;
  status: ScanStatus;
  findings: ScanFinding[];
  summary: string;
}

export interface ScanProgress {
  scan_id: string;
  step_label: string;
  step_index: number;
  total_steps: number;
  findings_so_far: number;
}

export interface ScoreReason {
  label: string;
  impact: number;
  severity: "INFO" | "LOW" | "MEDIUM" | "HIGH" | "CRITICAL";
}

export interface SecurityScore {
  score: number;
  reasons: ScoreReason[];
  calculated_at: string;
}

export interface NotificationSettings {
  enabled: boolean;
  min_severity: "INFO" | "LOW" | "MEDIUM" | "HIGH" | "CRITICAL";
}

export interface RetentionSettings {
  events_days: number;
  process_snapshots_days: number;
  port_snapshots_days: number;
}

export interface RetentionCleanupResult {
  events_deleted: number;
  process_snapshots_deleted: number;
  port_snapshots_deleted: number;
}

export const CUSTOM_SCAN_STEP_OPTIONS: { key: string; label: string }[] = [
  { key: "ports", label: "Open ports" },
  { key: "firewall", label: "Firewall rules" },
  { key: "startup", label: "Startup entries" },
  { key: "processes", label: "Running processes" },
  { key: "services", label: "Services" },
  { key: "adapters", label: "Network adapters" },
  { key: "files", label: "Recent file events" },
];
