use crate::commands::system::SysHandle;
use crate::models::AppError;
use serde::{Deserialize, Serialize};
use sysinfo::Pid;

/// A single row in the process table. Fields that require a
/// Windows-specific API (Authenticode signature status, integrity
/// level, live per-process network connection count) are `None` on
/// this cross-platform pass — the `windows` crate adapter that fills
/// them in from `WinVerifyTrust` / `NtQueryInformationProcess` lands
/// with the rest of the Windows-only code, not faked here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub name: String,
    pub exe_path: Option<String>,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub start_time_unix: u64,
    pub publisher: Option<String>,
    pub signed: Option<bool>,
    pub integrity_level: Option<String>,
    pub network_connection_count: Option<u32>,
    pub sha256: Option<String>,
}

#[tauri::command]
pub fn list_processes(sys_handle: tauri::State<SysHandle>) -> Result<Vec<ProcessInfo>, String> {
    let mut sys = sys_handle.0.lock().map_err(|e| e.to_string())?;
    sys.refresh_processes();

    let processes = sys
        .processes()
        .iter()
        .map(|(pid, proc_)| ProcessInfo {
            pid: pid.as_u32(),
            parent_pid: proc_.parent().map(|p| p.as_u32()),
            name: proc_.name().to_string(),
            exe_path: proc_.exe().map(|p| p.to_string_lossy().to_string()),
            cpu_percent: proc_.cpu_usage(),
            memory_bytes: proc_.memory(),
            start_time_unix: proc_.start_time(),
            // Windows-specific enrichment, filled in by a later
            // #[cfg(windows)] pass over the same rows:
            publisher: None,
            signed: None,
            integrity_level: None,
            network_connection_count: None,
            sha256: None,
        })
        .collect();

    Ok(processes)
}

/// Explicit, confirmed-by-the-user termination only — never called
/// automatically by any scan or background task. The frontend must
/// show a confirmation dialog before invoking this command.
#[tauri::command]
pub fn terminate_process(
    sys_handle: tauri::State<SysHandle>,
    pid: u32,
) -> Result<(), AppError> {
    let mut sys = sys_handle.0.lock().map_err(|e| AppError {
        code: "LOCK_ERROR".into(),
        message: e.to_string(),
        details: None,
        recoverable: true,
    })?;
    sys.refresh_processes();

    match sys.process(Pid::from_u32(pid)) {
        Some(proc_) => {
            if proc_.kill() {
                Ok(())
            } else {
                Err(AppError {
                    code: "TERMINATE_FAILED".into(),
                    message: "The OS refused to terminate this process.".into(),
                    details: Some(
                        "This can happen for protected system processes or due to insufficient privileges.".into(),
                    ),
                    recoverable: true,
                })
            }
        }
        None => Err(AppError {
            code: "PROCESS_NOT_FOUND".into(),
            message: format!("No running process with PID {pid}."),
            details: None,
            recoverable: false,
        }),
    }
}
