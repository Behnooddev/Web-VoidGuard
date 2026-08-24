use crate::db::Db;
use crate::models::RetentionSettings;
use rusqlite::params;

const SETTINGS_KEY: &str = "retention_settings";

#[tauri::command]
pub fn get_retention_settings(db: tauri::State<Db>) -> Result<RetentionSettings, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let stored: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![SETTINGS_KEY],
            |row| row.get(0),
        )
        .ok();

    Ok(stored
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default())
}

#[tauri::command]
pub fn set_retention_settings(
    db: tauri::State<Db>,
    settings: RetentionSettings,
) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![SETTINGS_KEY, serde_json::to_string(&settings).unwrap()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Deletes rows older than each table's configured window. Run
/// on-demand from Settings ("Clean up now") and also on app startup
/// (see `main.rs`) so the databases doesn't just grow forever between
/// visits to that page. A `0` days setting skips that table entirely.
/// Returns how many rows were removed from each table, so the UI can
/// show something more concrete than "done".
#[tauri::command]
pub fn run_retention_cleanup(db: tauri::State<Db>) -> Result<RetentionCleanupResult, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let stored: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![SETTINGS_KEY],
            |row| row.get(0),
        )
        .ok();
    let settings: RetentionSettings = stored
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let events_deleted = delete_older_than(&conn, "events", "timestamp", settings.events_days)?;
    let process_deleted = delete_older_than(
        &conn,
        "process_snapshots",
        "timestamp",
        settings.process_snapshots_days,
    )?;
    let port_deleted = delete_older_than(
        &conn,
        "port_snapshots",
        "timestamp",
        settings.port_snapshots_days,
    )?;

    Ok(RetentionCleanupResult {
        events_deleted,
        process_snapshots_deleted: process_deleted,
        port_snapshots_deleted: port_deleted,
    })
}

fn delete_older_than(
    conn: &rusqlite::Connection,
    table: &str,
    timestamp_col: &str,
    days: u32,
) -> Result<u32, String> {
    if days == 0 {
        return Ok(0);
    }
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(days as i64)).to_rfc3339();
    // `table`/`timestamp_col` are fixed, hardcoded call sites above —
    // never user input — so building this string is safe; the cutoff
    // value itself still goes through a bound parameter.
    let sql = format!("DELETE FROM {table} WHERE {timestamp_col} < ?1");
    let deleted = conn
        .execute(&sql, params![cutoff])
        .map_err(|e| e.to_string())?;
    Ok(deleted as u32)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RetentionCleanupResult {
    pub events_deleted: u32,
    pub process_snapshots_deleted: u32,
    pub port_snapshots_deleted: u32,
}
