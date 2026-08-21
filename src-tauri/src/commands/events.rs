use crate::db::Db;
use crate::models::{EventCategory, Severity, SystemEvent};
use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

/// Inserts a normalized event and returns it. Called by monitoring
/// subsystems (process/network/firewall/file watchers in later
/// phases) as well as directly from the frontend for manual test
/// events during Phase 1 development.
pub fn record_event(
    db: &Db,
    category: EventCategory,
    severity: Severity,
    source: &str,
    description: &str,
    target: Option<String>,
) -> Result<SystemEvent, String> {
    let event = SystemEvent {
        id: Uuid::new_v4().to_string(),
        timestamp: Utc::now(),
        category,
        severity,
        source: source.to_string(),
        description: description.to_string(),
        target,
        previous_state: None,
        new_state: None,
        related_process: None,
        related_file: None,
        risk_score: 0,
    };

    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO events (id, timestamp, category, severity, source, description, target,
            previous_state, new_state, related_process, related_file, risk_score)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            event.id,
            event.timestamp.to_rfc3339(),
            serde_json::to_string(&event.category).unwrap(),
            serde_json::to_string(&event.severity).unwrap(),
            event.source,
            event.description,
            event.target,
            event.previous_state,
            event.new_state,
            event.related_process,
            event.related_file,
            event.risk_score,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(event)
}

#[tauri::command]
pub fn get_recent_events(db: tauri::State<Db>, limit: u32) -> Result<Vec<SystemEvent>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, timestamp, category, severity, source, description, target,
                previous_state, new_state, related_process, related_file, risk_score
             FROM events ORDER BY timestamp DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![limit], |row| {
            let category: String = row.get(2)?;
            let severity: String = row.get(3)?;
            Ok(SystemEvent {
                id: row.get(0)?,
                timestamp: row
                    .get::<_, String>(1)?
                    .parse()
                    .unwrap_or_else(|_| Utc::now()),
                category: serde_json::from_str(&category).unwrap(),
                severity: serde_json::from_str(&severity).unwrap(),
                source: row.get(4)?,
                description: row.get(5)?,
                target: row.get(6)?,
                previous_state: row.get(7)?,
                new_state: row.get(8)?,
                related_process: row.get(9)?,
                related_file: row.get(10)?,
                risk_score: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}
