import { invoke } from "@tauri-apps/api/tauri";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AuditEntry, SystemEvent, SystemMetrics } from "@/types";
import type { AppError, ProcessInfo } from "@/types/process";

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
  return invoke("terminate_process", { pid }).catch((e: AppError | string) => {
    throw typeof e === "string" ? { code: "UNKNOWN", message: e, details: null, recoverable: true } : e;
  });
}

export function onSystemMetrics(
  cb: (metrics: SystemMetrics) => void
): Promise<UnlistenFn> {
  return listen<SystemMetrics>("system-metrics", (event) => cb(event.payload));
}
