export type FileChangeType = "CREATED" | "MODIFIED" | "DELETED";

export interface FileEvent {
  id: string;
  timestamp: string;
  path: string;
  change_type: FileChangeType;
  sha256: string | null;
  size_bytes: number | null;
}

export interface WatchScope {
  path: string;
  recursive: boolean;
  label: string;
  built_in: boolean;
}

export type StartupClassification = "KNOWN" | "UNKNOWN" | "SUSPICIOUS";
export type StartupLocationType =
  | "REGISTRY_RUN"
  | "REGISTRY_RUN_ONCE"
  | "STARTUP_FOLDER"
  | "SCHEDULED_TASK";

export interface StartupEntry {
  id: string;
  name: string;
  command: string;
  location_type: StartupLocationType;
  classification: StartupClassification;
  evidence: string[];
  first_seen: string;
  last_seen: string;
}

export type Confidence = "LOW" | "MEDIUM" | "HIGH";

export interface RiskFinding {
  id: string;
  timestamp: string;
  title: string;
  description: string;
  severity: "INFO" | "LOW" | "MEDIUM" | "HIGH" | "CRITICAL";
  confidence: Confidence;
  evidence: string[];
  remediation: string | null;
  related_event_ids: string[];
}
