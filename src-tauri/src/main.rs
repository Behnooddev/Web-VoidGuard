#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod models;

use commands::system::{get_system_metrics, init_handle, SysHandle};
use commands::events::{get_recent_events, record_event};
use commands::audit::get_audit_log;
use commands::process::{list_processes, terminate_process};
use commands::ports::{close_port, list_listening_ports, open_port, terminate_port_owner};
use commands::network::list_network_adapters;
use commands::services::{change_service_startup_type, control_service, list_services};
use commands::files::{get_recent_file_events, get_watch_scopes, init_and_start_watching, init_handle as init_file_watcher_handle, FileWatcherHandle};
use commands::startup::{list_startup_entries, remove_startup_entry};
use commands::risk::{get_recent_risk_findings, run_risk_analysis};
use commands::firewall::{create_firewall_rule, delete_firewall_rule, list_firewall_rules, set_firewall_rule_enabled};
use commands::dns::change_dns;
use models::{EventCategory, Severity};
use tauri::Manager;

fn main() {
    let db = db::init().expect("failed to initialize database");
    let sys_handle = init_handle();
    let file_watcher_handle = init_file_watcher_handle();

    tauri::Builder::default()
        .manage(db)
        .manage(sys_handle)
        .manage(file_watcher_handle)
        .invoke_handler(tauri::generate_handler![
            get_system_metrics,
            get_recent_events,
            get_audit_log,
            list_processes,
            terminate_process,
            list_listening_ports,
            terminate_port_owner,
            open_port,
            close_port,
            list_network_adapters,
            list_services,
            control_service,
            change_service_startup_type,
            get_watch_scopes,
            get_recent_file_events,
            list_startup_entries,
            remove_startup_entry,
            run_risk_analysis,
            get_recent_risk_findings,
            list_firewall_rules,
            create_firewall_rule,
            set_firewall_rule_enabled,
            delete_firewall_rule,
            change_dns,
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
                    "VoidGuard started",
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

            // Start file integrity watching on the small, built-in
            // set of security-sensitive locations (plus any
            // user-configured scopes already in the DB). Errors here
            // are non-fatal to app startup — logged as a low-severity
            // event instead (see files::start_watcher).
            {
                let db_state = handle.state::<db::Db>();
                let watcher_state = handle.state::<commands::files::FileWatcherHandle>();
                if let Err(e) = commands::files::init_and_start_watching(&db_state, &watcher_state) {
                    eprintln!("File watcher did not start: {}", e.message);
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running VoidGuard");
}
