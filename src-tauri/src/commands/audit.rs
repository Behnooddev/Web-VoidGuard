use crate::db::Db;
use crate::models::{AuditEntry, AuditResult};
use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Writes one append-only audit entry. Every privileged operation in
/// later phases (DNS change, firewall rule edit, service start/stop,
/// process termination) must call this, success or failure, so the
/// audit log stays a complete record of what the app did.
pub fn record_audit(
    db: &Db,
    action: &str,
    target: &str,
    before: Option<String>,
    after: Option<String>,
    result: AuditResult,
    source: &str,
) -> Result<AuditEntry, String> {
    let entry = AuditEntry {
        id: Uuid::new_v4().to_string(),
        timestamp: Utc::now(),
        user: whoami_user(),
        action: action.to_string(),
        target: target.to_string(),
        before,
        after,
        result,
        source: source.to_string(),
        app_version: APP_VERSION.to_string(),
    };

    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO audit_logs (id, timestamp, user, action, target, before, after, result, source, app_version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            entry.id,
            entry.timestamp.to_rfc3339(),
            entry.user,
            entry.action,
            entry.target,
            entry.before,
            entry.after,
            serde_json::to_string(&entry.result).unwrap(),
            entry.source,
            entry.app_version,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(entry)
}

#[tauri::command]
pub fn get_audit_log(db: tauri::State<Db>, limit: u32) -> Result<Vec<AuditEntry>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, timestamp, user, action, target, before, after, result, source, app_version
             FROM audit_logs ORDER BY timestamp DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![limit], |row| {
            let result: String = row.get(7)?;
            Ok(AuditEntry {
                id: row.get(0)?,
                timestamp: row
                    .get::<_, String>(1)?
                    .parse()
                    .unwrap_or_else(|_| Utc::now()),
                user: row.get(2)?,
                action: row.get(3)?,
                target: row.get(4)?,
                before: row.get(5)?,
                after: row.get(6)?,
                result: serde_json::from_str(&result).unwrap(),
                source: row.get(8)?,
                app_version: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn whoami_user() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown".to_string())
}
