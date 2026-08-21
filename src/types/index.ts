export type Severity = "INFO" | "LOW" | "MEDIUM" | "HIGH" | "CRITICAL";

export type EventCategory =
  | "FILE_CREATED"
  | "FILE_DELETED"
  | "FILE_MODIFIED"
  | "PROCESS_STARTED"
  | "PROCESS_STOPPED"
  | "SERVICE_CHANGED"
  | "PORT_OPENED"
  | "PORT_CLOSED"
  | "DNS_CHANGED"
  | "FIREWALL_CHANGED"
  | "NETWORK_CHANGED"
  | "STARTUP_CHANGED"
  | "SECURITY_SETTING_CHANGED"
  | "SYSTEM_STARTED";

export interface SystemEvent {
  id: string;
  timestamp: string;
  category: EventCategory;
  severity: Severity;
  source: string;
  description: string;
  target: string | null;
  previous_state: string | null;
  new_state: string | null;
  related_process: string | null;
  related_file: string | null;
  risk_score: number;
}

export interface DiskMetrics {
  mount_point: string;
  used_bytes: number;
  total_bytes: number;
}

export interface SystemMetrics {
  timestamp: string;
  cpu_usage_percent: number;
  ram_used_bytes: number;
  ram_total_bytes: number;
  disks: DiskMetrics[];
  process_count: number;
  uptime_seconds: number;
  host_name: string | null;
  os_version: string | null;
}

export type AuditResult = "SUCCESS" | "FAILURE" | "DENIED";

export interface AuditEntry {
  id: string;
  timestamp: string;
  user: string;
  action: string;
  target: string;
  before: string | null;
  after: string | null;
  result: AuditResult;
  source: string;
  app_version: string;
}
