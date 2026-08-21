use crate::models::{DiskMetrics, SystemMetrics};
use chrono::Utc;
use std::sync::Mutex;
use sysinfo::{Disks, System};

/// Long-lived sysinfo handle. Refreshing in place (rather than
/// constructing a new System each tick) is what sysinfo's CPU
/// percentage calculation requires, and it avoids repeated heavy
/// process-table walks.
pub struct SysHandle(pub Mutex<System>);

pub fn init_handle() -> SysHandle {
    let mut sys = System::new_all();
    sys.refresh_all();
    SysHandle(Mutex::new(sys))
}

#[tauri::command]
pub fn get_system_metrics(sys_handle: tauri::State<SysHandle>) -> Result<SystemMetrics, String> {
    let mut sys = sys_handle.0.lock().map_err(|e| e.to_string())?;
    sys.refresh_cpu();
    sys.refresh_memory();
    sys.refresh_processes();

    let cpu_usage_percent = sys.global_cpu_info().cpu_usage();
    let ram_used_bytes = sys.used_memory();
    let ram_total_bytes = sys.total_memory();
    let process_count = sys.processes().len();
    let uptime_seconds = System::uptime();
    let host_name = System::host_name();
    let os_version = System::long_os_version();

    let disks = Disks::new_with_refreshed_list()
        .iter()
        .map(|d| DiskMetrics {
            mount_point: d.mount_point().to_string_lossy().to_string(),
            used_bytes: d.total_space().saturating_sub(d.available_space()),
            total_bytes: d.total_space(),
        })
        .collect();

    Ok(SystemMetrics {
        timestamp: Utc::now(),
        cpu_usage_percent,
        ram_used_bytes,
        ram_total_bytes,
        disks,
        process_count,
        uptime_seconds,
        host_name,
        os_version,
    })
}
