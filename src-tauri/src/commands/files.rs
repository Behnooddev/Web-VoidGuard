use crate::commands::events::record_event;
use crate::db::Db;
use crate::models::{AppError, EventCategory, FileChangeType, FileEvent, Severity, WatchScope};
use chrono::Utc;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use rusqlite::params;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::Mutex;
use uuid::Uuid;

/// Holds the live filesystem watcher so it isn't dropped (and stopped)
/// when `init_watcher` returns. One watcher instance, re-configured
/// (watch/unwatch calls) rather than recreated when scopes change.
pub struct FileWatcherHandle(pub Mutex<Option<RecommendedWatcher>>);

pub fn init_handle() -> FileWatcherHandle {
    FileWatcherHandle(Mutex::new(None))
}

/// The small, conservative default watch list: security-sensitive
/// locations only — not the whole disk. Matches the spec's "monitor
/// security-sensitive locations and user-configured locations, not
/// the entire disk continuously" requirement.
pub fn default_watch_scopes() -> Vec<WatchScope> {
    #[cfg(windows)]
    {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
        let userprofile = std::env::var("USERPROFILE").unwrap_or_default();
        let programdata = std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".into());

        vec![
            WatchScope {
                path: format!(r"{windir}\System32\drivers\etc"),
                recursive: false,
                label: "hosts file & DNS overrides".into(),
                built_in: true,
            },
            WatchScope {
                path: format!(
                    r"{userprofile}\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Startup"
                ),
                recursive: true,
                label: "user Startup folder".into(),
                built_in: true,
            },
            WatchScope {
                path: format!(
                    r"{programdata}\Microsoft\Windows\Start Menu\Programs\StartUp"
                ),
                recursive: true,
                label: "all-users Startup folder".into(),
                built_in: true,
            },
        ]
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[tauri::command]
pub fn get_watch_scopes(db: tauri::State<Db>) -> Result<Vec<WatchScope>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT path, recursive, label, built_in FROM watch_scopes ORDER BY built_in DESC, path")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(WatchScope {
                path: row.get(0)?,
                recursive: row.get::<_, i64>(1)? != 0,
                label: row.get(2)?,
                built_in: row.get::<_, i64>(3)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Seeds `watch_scopes` with the built-in defaults the first time the
/// app runs (idempotent — `INSERT OR IGNORE`), then starts watching
/// every scope currently in the table.
pub fn init_and_start_watching(db: &Db, handle: &FileWatcherHandle) -> Result<(), AppError> {
    {
        let conn = db.0.lock().map_err(lock_err)?;
        for scope in default_watch_scopes() {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO watch_scopes (path, recursive, label, built_in) VALUES (?1, ?2, ?3, 1)",
                params![scope.path, scope.recursive as i64, scope.label],
            );
        }
    }

    let scopes = {
        let conn = db.0.lock().map_err(lock_err)?;
        let mut stmt = conn
            .prepare("SELECT path, recursive FROM watch_scopes")
            .map_err(|e| db_err(&e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0))
            })
            .map_err(|e| db_err(&e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| db_err(&e.to_string()))?
    };

    start_watcher(db, handle, &scopes)
}

fn start_watcher(
    db: &Db,
    handle: &FileWatcherHandle,
    scopes: &[(String, bool)],
) -> Result<(), AppError> {
    let (tx, rx) = channel::<notify::Result<notify::Event>>();

    let mut watcher: RecommendedWatcher = notify::recommended_watcher(tx).map_err(|e| AppError {
        code: "WATCHER_INIT_FAILED".into(),
        message: "Could not start the file integrity watcher.".into(),
        details: Some(e.to_string()),
        recoverable: true,
    })?;

    for (path, recursive) in scopes {
        let mode = if *recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        if let Err(e) = watcher.watch(&PathBuf::from(path), mode) {
            // A missing/inaccessible path shouldn't abort the whole
            // watcher — log it as a low-severity event and continue
            // with the remaining scopes.
            let _ = record_event(
                db,
                EventCategory::SecuritySettingChanged,
                Severity::Low,
                "files",
                &format!("Could not watch '{path}': {e}"),
                Some(path.clone()),
            );
        }
    }

    *handle.0.lock().map_err(lock_err)? = Some(watcher);

    // Forward filesystem events into the events table on a plain
    // OS thread — notify's channel is sync, not async, and this
    // avoids pulling the whole watcher onto the Tokio runtime.
    let db_conn_path = crate::db::app_data_dir().join("voidguard.db");
    std::thread::spawn(move || {
        let conn = match rusqlite::Connection::open(&db_conn_path) {
            Ok(c) => c,
            Err(_) => return,
        };
        for res in rx {
            if let Ok(event) = res {
                handle_fs_event(&conn, event);
            }
        }
    });

    Ok(())
}

fn handle_fs_event(conn: &rusqlite::Connection, event: notify::Event) {
    use notify::EventKind;
    let change_type = match event.kind {
        EventKind::Create(_) => FileChangeType::Created,
        EventKind::Modify(_) => FileChangeType::Modified,
        EventKind::Remove(_) => FileChangeType::Deleted,
        _ => return, // access/other events aren't integrity-relevant
    };

    for path in event.paths {
        let path_str = path.to_string_lossy().to_string();
        let (sha256, size_bytes) = if change_type != FileChangeType::Deleted {
            hash_file(&path)
        } else {
            (None, None)
        };

        let file_event = FileEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            path: path_str.clone(),
            change_type,
            sha256,
            size_bytes,
        };

        let _ = conn.execute(
            "INSERT INTO file_events (id, timestamp, path, change_type, sha256, size_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                file_event.id,
                file_event.timestamp.to_rfc3339(),
                file_event.path,
                serde_json::to_string(&file_event.change_type).unwrap(),
                file_event.sha256,
                file_event.size_bytes,
            ],
        );

        let category = match change_type {
            FileChangeType::Created => EventCategory::FileCreated,
            FileChangeType::Modified => EventCategory::FileModified,
            FileChangeType::Deleted => EventCategory::FileDeleted,
        };
        let _ = conn.execute(
            "INSERT INTO events (id, timestamp, category, severity, source, description, target, risk_score)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                Uuid::new_v4().to_string(),
                Utc::now().to_rfc3339(),
                serde_json::to_string(&category).unwrap(),
                serde_json::to_string(&Severity::Low).unwrap(),
                "files",
                format!("{:?} in a watched location: {}", change_type, path_str),
                path_str,
                0,
            ],
        );
    }
}

fn hash_file(path: &PathBuf) -> (Option<String>, Option<u64>) {
    match std::fs::read(path) {
        Ok(bytes) => {
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let digest = hasher.finalize();
            (
                Some(digest.iter().map(|b| format!("{b:02x}")).collect()),
                Some(bytes.len() as u64),
            )
        }
        // File may have already been deleted/moved/locked by the time
        // we get to read it — not an error worth surfacing per-event.
        Err(_) => (None, None),
    }
}

#[tauri::command]
pub fn get_recent_file_events(db: tauri::State<Db>, limit: u32) -> Result<Vec<FileEvent>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, timestamp, path, change_type, sha256, size_bytes
             FROM file_events ORDER BY timestamp DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![limit], |row| {
            let change_type: String = row.get(3)?;
            Ok(FileEvent {
                id: row.get(0)?,
                timestamp: row
                    .get::<_, String>(1)?
                    .parse()
                    .unwrap_or_else(|_| Utc::now()),
                path: row.get(2)?,
                change_type: serde_json::from_str(&change_type).unwrap(),
                sha256: row.get(4)?,
                size_bytes: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn lock_err<T>(_: T) -> AppError {
    AppError {
        code: "LOCK_ERROR".into(),
        message: "Internal lock error.".into(),
        details: None,
        recoverable: true,
    }
}

fn db_err(msg: &str) -> AppError {
    AppError {
        code: "DB_ERROR".into(),
        message: "Database error while setting up file watching.".into(),
        details: Some(msg.to_string()),
        recoverable: true,
    }
}
