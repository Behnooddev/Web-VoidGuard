import { invoke } from "@tauri-apps/api/tauri";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AuditEntry, SystemEvent, SystemMetrics } from "@/types";
import type { AppError, ProcessInfo } from "@/types/process";
import type { ListeningPort, PortRuleRequest } from "@/types/ports";
import type {
  ChangeStartupTypeRequest,
  DnsSettingsRequest,
  NetworkAdapter,
  ServiceActionRequest,
  ServiceInfo,
} from "@/types/system_control";
import type { FileEvent, RiskFinding, StartupEntry, WatchScope } from "@/types/monitoring";
import type {
  CreateFirewallRuleRequest,
  FirewallRule,
  SetFirewallRuleEnabledRequest,
} from "@/types/firewall";

/**
 * Every function here corresponds 1:1 to a #[tauri::command] in the
 * Rust backend. The frontend never constructs shell commands or raw
 * strings that reach the OS directly — it only ever sends typed
 * request objects, matching the security boundary in ARCHITECTURE.md.
 */

export function getSystemMetrics(): Promise<SystemMetrics> {
  return invoke("get_system_metrics");
}

export function getRecentEvents(limit = 50): Promise<SystemEvent[]> {
  return invoke("get_recent_events", { limit });
}

export function getAuditLog(limit = 50): Promise<AuditEntry[]> {
  return invoke("get_audit_log", { limit });
}

export function listProcesses(): Promise<ProcessInfo[]> {
  return invoke("list_processes");
}

/**
 * Terminates a process. The caller (UI) MUST show an explicit
 * confirmation dialog before calling this — see ProcessesPage.tsx.
 * Rejects with a structured AppError, never a bare string, on failure.
 */
export function terminateProcess(pid: number): Promise<void> {
  return invoke<void>("terminate_process", { pid }).catch((e: AppError | string) => {
    throw typeof e === "string" ? { code: "UNKNOWN", message: e, details: null, recoverable: true } : e;
  });
}

function wrapAppError<T>(p: Promise<T>): Promise<T> {
  return p.catch((e: AppError | string) => {
    throw typeof e === "string"
      ? { code: "UNKNOWN", message: e, details: null, recoverable: true }
      : e;
  });
}

export function listListeningPorts(): Promise<ListeningPort[]> {
  return wrapAppError(invoke("list_listening_ports"));
}

/** Terminates the process bound to a port. Requires prior UI confirmation. */
export function terminatePortOwner(port: number, pid: number): Promise<void> {
  return wrapAppError(invoke("terminate_port_owner", { port, pid }));
}

/** Allows a port through Windows Firewall. Requires prior UI confirmation. */
export function openPort(req: PortRuleRequest): Promise<void> {
  return wrapAppError(invoke("open_port", { req }));
}

/** Blocks/removes the allow rule for a port. Requires prior UI confirmation. */
export function closePort(req: PortRuleRequest): Promise<void> {
  return wrapAppError(invoke("close_port", { req }));
}

export function listNetworkAdapters(): Promise<NetworkAdapter[]> {
  return wrapAppError(invoke("list_network_adapters"));
}

export function listServices(): Promise<ServiceInfo[]> {
  return wrapAppError(invoke("list_services"));
}

/** Start/stop/restart a service. Requires prior UI confirmation — stronger for protected services. */
export function controlService(req: ServiceActionRequest): Promise<void> {
  return wrapAppError(invoke("control_service", { req }));
}

export function changeServiceStartupType(req: ChangeStartupTypeRequest): Promise<void> {
  return wrapAppError(invoke("change_service_startup_type", { req }));
}

export function getWatchScopes(): Promise<WatchScope[]> {
  return invoke("get_watch_scopes");
}

export function getRecentFileEvents(limit = 50): Promise<FileEvent[]> {
  return invoke("get_recent_file_events", { limit });
}

export function listStartupEntries(): Promise<StartupEntry[]> {
  return wrapAppError(invoke("list_startup_entries"));
}

/** Removes a startup persistence entry. Requires prior UI confirmation. */
export function removeStartupEntry(id: string): Promise<void> {
  return wrapAppError(invoke("remove_startup_entry", { id }));
}

export function runRiskAnalysis(): Promise<RiskFinding[]> {
  return invoke("run_risk_analysis");
}

export function getRecentRiskFindings(limit = 20): Promise<RiskFinding[]> {
  return invoke("get_recent_risk_findings", { limit });
}

export function listFirewallRules(): Promise<FirewallRule[]> {
  return wrapAppError(invoke("list_firewall_rules"));
}

/** Creates a new Windows Firewall rule. Requires prior UI confirmation. */
export function createFirewallRule(req: CreateFirewallRuleRequest): Promise<void> {
  return wrapAppError(invoke("create_firewall_rule", { req }));
}

export function setFirewallRuleEnabled(req: SetFirewallRuleEnabledRequest): Promise<void> {
  return wrapAppError(invoke("set_firewall_rule_enabled", { req }));
}

/** Deletes a firewall rule VoidGuard created. Requires prior UI confirmation. */
export function deleteFirewallRule(name: string): Promise<void> {
  return wrapAppError(invoke("delete_firewall_rule", { name }));
}

/** Applies per-interface DNS settings. Requires prior UI confirmation. */
export function changeDns(req: DnsSettingsRequest): Promise<void> {
  return wrapAppError(invoke("change_dns", { req }));
}

export function onSystemMetrics(
  cb: (metrics: SystemMetrics) => void
): Promise<UnlistenFn> {
  return listen<SystemMetrics>("system-metrics", (event) => cb(event.payload));
}
