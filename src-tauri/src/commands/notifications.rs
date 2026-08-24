use crate::db::Db;
use crate::models::NotificationSettings;
use rusqlite::params;

const SETTINGS_KEY: &str = "notification_settings";

/// Notification *preferences* live here; the actual OS toast is
/// raised from the frontend via Tauri's Notification API once it
/// sees a new high-severity event, gated by these settings — see
/// `src/components/NotificationManager.tsx`. Keeping the OS-level
/// dispatch in the frontend avoids a second, Rust-side notification
/// pathway that could drift out of sync with what the Events page
/// already shows.
#[tauri::command]
pub fn get_notification_settings(db: tauri::State<Db>) -> Result<NotificationSettings, String> {
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
pub fn set_notification_settings(
    db: tauri::State<Db>,
    settings: NotificationSettings,
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
