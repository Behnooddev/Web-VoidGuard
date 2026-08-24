use crate::commands::audit::record_audit;
use crate::commands::events::record_event;
use crate::commands::system::SysHandle;
use crate::db::Db;
use crate::models::{
    AuditResult, EventCategory, ScanFinding, ScanProgress, ScanResult, ScanStatus, ScanType,
    Severity,
};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::Manager;
use uuid::Uuid;

/// The full catalog of scan steps. `scan_type` picks a fixed subset;
/// `Custom` lets the caller pick any combination by key. Each closure
/// does real enumeration work (reusing the same commands the rest of
/// the app uses) and returns findings — there is no fake progress
/// here, `step_index`/`total_steps` map 1:1 to actual work performed.
struct ScanStep {
    key: &'static str,
    label: &'static str,
}

const ALL_STEPS: &[ScanStep] = &[
    ScanStep { key: "ports", label: "Checking listening ports" },
    ScanStep { key: "firewall", label: "Checking firewall rules" },
    ScanStep { key: "startup", label: "Checking startup entries" },
    ScanStep { key: "processes", label: "Checking running processes" },
    ScanStep { key: "services", label: "Checking services" },
    ScanStep { key: "adapters", label: "Checking network adapters" },
    ScanStep { key: "files", label: "Checking recent file events" },
];

fn steps_for(scan_type: ScanType, custom: &Option<Vec<String>>) -> Vec<&'static ScanStep> {
    let keys: Vec<&str> = if scan_type == ScanType::Custom {
        custom
            .as_ref()
            .map(|v| v.iter().map(String::as_str).collect())
            .unwrap_or_default()
    } else {
        match scan_type {
            ScanType::Quick => vec!["ports", "firewall", "startup"],
            ScanType::System => vec!["processes", "services", "startup", "firewall"],
            ScanType::Network => vec!["adapters", "ports"],
            ScanType::Startup => vec!["startup"],
            ScanType::Integrity => vec!["files"],
            ScanType::Custom => vec![],
        }
    };
    ALL_STEPS.iter().filter(|s| keys.contains(&s.key)).collect()
}

#[derive(Serialize, Deserialize)]
struct StoredSummary {
    findings: Vec<ScanFinding>,
    summary_text: String,
}

#[tauri::command]
pub fn run_scan(
    app: tauri::AppHandle,
    db: tauri::State<Db>,
    sys_handle: tauri::State<SysHandle>,
    scan_type: ScanType,
    custom_steps: Option<Vec<String>>,
) -> Result<ScanResult, String> {
    let scan_id = Uuid::new_v4().to_string();
    let started_at = Utc::now();
    let steps = steps_for(scan_type, &custom_steps);
    let total_steps = steps.len().max(1) as u32;

    let mut findings: Vec<ScanFinding> = Vec::new();

    for (i, step) in steps.iter().enumerate() {
        let step_findings = run_step(&app, &db, &sys_handle, step.key);
        findings.extend(step_findings);

        let _ = app.emit_all(
            "scan-progress",
            ScanProgress {
                scan_id: scan_id.clone(),
                step_label: step.label.to_string(),
                step_index: (i + 1) as u32,
                total_steps,
                findings_so_far: findings.len() as u32,
            },
        );
    }

    let summary_text = if findings.is_empty() {
        "No findings — nothing stood out in this scan.".to_string()
    } else {
        format!(
            "{} finding(s): {} high/critical, {} medium, {} low/info.",
            findings.len(),
            findings.iter().filter(|f| f.severity >= Severity::High).count(),
            findings.iter().filter(|f| f.severity == Severity::Medium).count(),
            findings
                .iter()
                .filter(|f| f.severity < Severity::Medium)
                .count(),
        )
    };

    let result = ScanResult {
        id: scan_id.clone(),
        scan_type,
        started_at,
        finished_at: Some(Utc::now()),
        status: ScanStatus::Completed,
        findings: findings.clone(),
        summary: summary_text.clone(),
    };

    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let stored = StoredSummary { findings, summary_text: summary_text.clone() };
        conn.execute(
            "INSERT INTO scan_results (id, scan_type, started_at, finished_at, status, findings_count, summary)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                result.id,
                serde_json::to_string(&result.scan_type).unwrap(),
                result.started_at.to_rfc3339(),
                result.finished_at.unwrap().to_rfc3339(),
                serde_json::to_string(&result.status).unwrap(),
                result.findings.len() as i64,
                serde_json::to_string(&stored).unwrap(),
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    let _ = record_audit(
        &db,
        "RUN_SCAN",
        &format!("{scan_type:?} scan"),
        None,
        Some(summary_text.clone()),
        AuditResult::Success,
        "scan",
    );
    let _ = record_event(
        &db,
        EventCategory::SecuritySettingChanged,
        if result.findings.iter().any(|f| f.severity >= Severity::High) {
            Severity::High
        } else {
            Severity::Info
        },
        "scan",
        &format!("{scan_type:?} scan completed: {summary_text}"),
        None,
    );

    Ok(result)
}

fn run_step(
    app: &tauri::AppHandle,
    db: &tauri::State<Db>,
    sys_handle: &tauri::State<SysHandle>,
    key: &str,
) -> Vec<ScanFinding> {
    match key {
        "ports" => {
            let ports = crate::commands::ports::list_listening_ports().unwrap_or_default();
            ports
                .into_iter()
                .filter(|p| !matches!(p.risk, crate::models::PortRisk::Low))
                .map(|p| ScanFinding {
                    label: format!("{:?} port {} is listening", p.protocol, p.port),
                    detail: format!(
                        "Owned by {} (PID {:?}) — risk: {:?}",
                        p.process_name.as_deref().unwrap_or("unknown"),
                        p.pid,
                        p.risk
                    ),
                    severity: match p.risk {
                        crate::models::PortRisk::High => Severity::High,
                        _ => Severity::Medium,
                    },
                })
                .collect()
        }
        "firewall" => {
            let state = app.state::<Db>();
            let rules = crate::commands::firewall::list_firewall_rules(state).unwrap_or_default();
            rules
                .into_iter()
                .filter(|r| !r.enabled)
                .map(|r| ScanFinding {
                    label: format!("Firewall rule '{}' is disabled", r.name),
                    detail: "A VoidGuard-managed rule is currently disabled.".into(),
                    severity: Severity::Low,
                })
                .collect()
        }
        "startup" => {
            let state = app.state::<Db>();
            let entries = crate::commands::startup::list_startup_entries(state).unwrap_or_default();
            entries
                .into_iter()
                .filter(|e| {
                    !matches!(e.classification, crate::models::StartupClassification::Known)
                })
                .map(|e| ScanFinding {
                    label: format!("Startup entry '{}' is {:?}", e.name, e.classification),
                    detail: e.evidence.join("; "),
                    severity: match e.classification {
                        crate::models::StartupClassification::Suspicious => Severity::High,
                        _ => Severity::Medium,
                    },
                })
                .collect()
        }
        "processes" => {
            let state_ref = app.state::<SysHandle>();
            let procs = crate::commands::process::list_processes(state_ref).unwrap_or_default();
            vec![ScanFinding {
                label: format!("{} processes currently running", procs.len()),
                detail: "Signature/publisher checking isn't implemented yet, so processes aren't individually flagged here.".into(),
                severity: Severity::Info,
            }]
        }
        "services" => {
            let services = crate::commands::services::list_services().unwrap_or_default();
            services
                .into_iter()
                .filter(|s| s.protected && s.status != crate::models::ServiceStatus::Running)
                .map(|s| ScanFinding {
                    label: format!("Protected service '{}' is not running", s.display_name),
                    detail: format!("Status: {:?}", s.status),
                    severity: Severity::High,
                })
                .collect()
        }
        "adapters" => {
            let adapters = crate::commands::network::list_network_adapters().unwrap_or_default();
            vec![ScanFinding {
                label: format!("{} network adapter(s) found", adapters.len()),
                detail: adapters
                    .iter()
                    .map(|a| format!("{} ({})", a.name, a.status))
                    .collect::<Vec<_>>()
                    .join(", "),
                severity: Severity::Info,
            }]
        }
        "files" => {
            let state = app.state::<Db>();
            let events = crate::commands::files::get_recent_file_events(state, 50).unwrap_or_default();
            if events.is_empty() {
                vec![]
            } else {
                vec![ScanFinding {
                    label: format!("{} file event(s) in watched locations recently", events.len()),
                    detail: "See the Files page for the full list.".into(),
                    severity: Severity::Low,
                }]
            }
        }
        _ => {
            let _ = db; // keep parameter used across all match arms uniformly
            let _ = sys_handle;
            vec![]
        }
    }
}

#[tauri::command]
pub fn get_recent_scans(db: tauri::State<Db>, limit: u32) -> Result<Vec<ScanResult>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, scan_type, started_at, finished_at, status, summary
             FROM scan_results ORDER BY started_at DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![limit], |row| {
            let scan_type: String = row.get(1)?;
            let status: String = row.get(4)?;
            let summary_raw: String = row.get(5)?;
            let stored: StoredSummary = serde_json::from_str(&summary_raw).unwrap_or(StoredSummary {
                findings: vec![],
                summary_text: summary_raw,
            });
            Ok(ScanResult {
                id: row.get(0)?,
                scan_type: serde_json::from_str(&scan_type).unwrap(),
                started_at: row.get::<_, String>(2)?.parse().unwrap_or_else(|_| Utc::now()),
                finished_at: row.get::<_, Option<String>>(3)?.and_then(|s| s.parse().ok()),
                status: serde_json::from_str(&status).unwrap(),
                findings: stored.findings,
                summary: stored.summary_text,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}
