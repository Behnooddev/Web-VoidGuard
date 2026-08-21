#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod models;

use commands::system::{get_system_metrics, init_handle, SysHandle};
use commands::events::{get_recent_events, record_event};
use commands::audit::get_audit_log;
use commands::process::{list_processes, terminate_process};
use models::{EventCategory, Severity};
use tauri::Manager;

fn main() {
    let db = db::init().expect("failed to initialize database");
    let sys_handle = init_handle();

    tauri::Builder::default()
        .manage(db)
        .manage(sys_handle)
        .invoke_handler(tauri::generate_handler![
            get_system_metrics,
            get_recent_events,
            get_audit_log,
            list_processes,
            terminate_process,
        ])
        .setup(|app| {
            // Record app start as the first event of the session and
            // begin the lightweight, interval-based metrics stream
            // that the dashboard subscribes to. This is timer-driven,
            // not a busy loop, and does no disk/process scanning
            // beyond the single sysinfo refresh per tick.
            let handle = app.handle();

            {
                let db_state = handle.state::<db::Db>();
                let _ = record_event(
                    &db_state,
                    EventCategory::SystemStarted,
                    Severity::Info,
                    "app",
                    "WinGuard started",
                    None,
                );
            }

            let emitter_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
                loop {
                    interval.tick().await;
                    let sys_state = emitter_handle.state::<SysHandle>();
                    if let Ok(metrics) = get_system_metrics(sys_state) {
                        let _ = emitter_handle.emit_all("system-metrics", metrics);
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running WinGuard");
}
