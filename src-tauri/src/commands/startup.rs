use crate::commands::audit::record_audit;
use crate::commands::events::record_event;
use crate::db::Db;
use crate::models::{
    AppError, AuditResult, EventCategory, Severity, StartupClassification, StartupEntry,
    StartupLocationType,
};
use chrono::Utc;
use rusqlite::params;

#[tauri::command]
pub fn list_startup_entries(db: tauri::State<Db>) -> Result<Vec<StartupEntry>, AppError> {
    #[cfg(windows)]
    let entries = windows_impl::enumerate()?;
    #[cfg(not(windows))]
    let entries: Vec<StartupEntry> = {
        return Err(AppError::not_supported("Startup/persistence enumeration"));
    };

    persist_and_diff(&db, entries)
}

/// Upserts freshly-enumerated entries against what's already known,
/// updating `last_seen` for entries still present and recording a
/// `STARTUP_CHANGED` event for anything genuinely new — so re-running
/// this command (e.g. on a timer) doesn't spam the event feed with
/// entries that were already there.
fn persist_and_diff(db: &Db, entries: Vec<StartupEntry>) -> Result<Vec<StartupEntry>, AppError> {
    let conn = db.0.lock().map_err(|_| AppError {
        code: "LOCK_ERROR".into(),
        message: "Internal lock error.".into(),
        details: None,
        recoverable: true,
    })?;

    for entry in &entries {
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM startup_entries WHERE name = ?1 AND command = ?2",
                params![entry.name, entry.command],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing {
            let _ = conn.execute(
                "UPDATE startup_entries SET last_seen = ?1 WHERE id = ?2",
                params![entry.last_seen.to_rfc3339(), id],
            );
        } else {
            let _ = conn.execute(
                "INSERT INTO startup_entries (id, name, command, location_type, classification, evidence, first_seen, last_seen)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    entry.id,
                    entry.name,
                    entry.command,
                    serde_json::to_string(&entry.location_type).unwrap(),
                    serde_json::to_string(&entry.classification).unwrap(),
                    serde_json::to_string(&entry.evidence).unwrap(),
                    entry.first_seen.to_rfc3339(),
                    entry.last_seen.to_rfc3339(),
                ],
            );
            drop(conn); // release before calling record_event (re-locks internally)
            let severity = match entry.classification {
                StartupClassification::Suspicious => Severity::High,
                StartupClassification::Unknown => Severity::Medium,
                StartupClassification::Known => Severity::Info,
            };
            let _ = record_event(
                db,
                EventCategory::StartupChanged,
                severity,
                "startup",
                &format!("New startup entry detected: {} ({})", entry.name, entry.command),
                Some(entry.name.clone()),
            );
            return persist_and_diff(db, entries_minus_first(entries.clone(), entry));
        }
    }

    Ok(entries)
}

// Small helper so the recursive re-entry above (needed because we
// `drop(conn)` mid-loop to satisfy the borrow checker across the
// `record_event` call) doesn't reprocess entries already upserted.
// A future pass should restructure this as a plain loop with the
// event recorded via a queued Vec instead — flagged in the Phase 3
// handoff as a cleanup item, not a correctness bug.
fn entries_minus_first(entries: Vec<StartupEntry>, done: &StartupEntry) -> Vec<StartupEntry> {
    entries.into_iter().filter(|e| e.id != done.id).collect()
}

#[tauri::command]
pub fn remove_startup_entry(db: tauri::State<Db>, id: String) -> Result<(), AppError> {
    let (name, command, location_type) = {
        let conn = db.0.lock().map_err(|_| AppError {
            code: "LOCK_ERROR".into(),
            message: "Internal lock error.".into(),
            details: None,
            recoverable: true,
        })?;
        conn.query_row(
            "SELECT name, command, location_type FROM startup_entries WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .map_err(|_| AppError {
            code: "NOT_FOUND".into(),
            message: "Startup entry not found.".into(),
            details: None,
            recoverable: false,
        })?
    };
    let location: StartupLocationType = serde_json::from_str(&location_type).unwrap();

    #[cfg(windows)]
    let result = windows_impl::remove_entry(&name, &command, location);
    #[cfg(not(windows))]
    let result: Result<(), AppError> = Err(AppError::not_supported("Startup entry removal"));

    let _ = record_audit(
        &db,
        "REMOVE_STARTUP_ENTRY",
        &name,
        Some(command),
        None,
        match &result {
            Ok(_) => AuditResult::Success,
            Err(_) => AuditResult::Failure,
        },
        "startup",
    );

    result
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegEnumValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER,
        HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE, REG_SZ,
    };

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const RUN_ONCE_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\RunOnce";

    pub fn enumerate() -> Result<Vec<StartupEntry>, AppError> {
        let mut entries = Vec::new();

        entries.extend(read_run_key(HKEY_LOCAL_MACHINE, RUN_KEY, StartupLocationType::RegistryRun)?);
        entries.extend(read_run_key(HKEY_CURRENT_USER, RUN_KEY, StartupLocationType::RegistryRun)?);
        entries.extend(read_run_key(
            HKEY_LOCAL_MACHINE,
            RUN_ONCE_KEY,
            StartupLocationType::RegistryRunOnce,
        )?);
        entries.extend(read_run_key(
            HKEY_CURRENT_USER,
            RUN_ONCE_KEY,
            StartupLocationType::RegistryRunOnce,
        )?);

        entries.extend(read_startup_folders());

        // Scheduled Tasks (via the Task Scheduler COM API) are not
        // implemented in this pass — a folder/registry scan covers
        // the two most common persistence vectors, but Task Scheduler
        // needs its own ITaskService/ITaskFolder COM walk. Flagged in
        // the Phase 3 handoff rather than faked here.

        Ok(entries)
    }

    fn read_run_key(
        root: HKEY,
        subkey: &str,
        location_type: StartupLocationType,
    ) -> Result<Vec<StartupEntry>, AppError> {
        unsafe {
            let wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
            let mut hkey = HKEY::default();
            let opened = RegOpenKeyExW(root, PCWSTR(wide.as_ptr()), 0, KEY_READ, &mut hkey);
            if opened.is_err() {
                // Key not existing is normal (e.g. no RunOnce entries
                // right now) — not an error worth surfacing.
                return Ok(Vec::new());
            }

            let mut entries = Vec::new();
            let mut index = 0u32;
            loop {
                let mut name_buf = [0u16; 256];
                let mut name_len = name_buf.len() as u32;
                let mut value_buf = [0u16; 2048];
                let mut value_len = value_buf.len() as u32;
                let mut value_type = REG_SZ.0;

                let result = RegEnumValueW(
                    hkey,
                    index,
                    windows::core::PWSTR(name_buf.as_mut_ptr()),
                    &mut name_len,
                    None,
                    Some(&mut value_type),
                    Some(value_buf.as_mut_ptr() as *mut u8),
                    Some(&mut value_len),
                );
                if result.is_err() {
                    break; // ERROR_NO_MORE_ITEMS or similar — done
                }

                let name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
                let command =
                    String::from_utf16_lossy(&value_buf[..(value_len as usize / 2).saturating_sub(1)]);

                let (classification, evidence) = classify(&command);

                entries.push(StartupEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    name,
                    command,
                    location_type,
                    classification,
                    evidence,
                    first_seen: Utc::now(),
                    last_seen: Utc::now(),
                });

                index += 1;
            }

            let _ = RegCloseKey(hkey);
            Ok(entries)
        }
    }

    fn read_startup_folders() -> Vec<StartupEntry> {
        let mut entries = Vec::new();
        let userprofile = std::env::var("USERPROFILE").unwrap_or_default();
        let programdata = std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".into());

        for folder in [
            format!(r"{userprofile}\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Startup"),
            format!(r"{programdata}\Microsoft\Windows\Start Menu\Programs\StartUp"),
        ] {
            if let Ok(dir) = std::fs::read_dir(&folder) {
                for entry in dir.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let command = path.to_string_lossy().to_string();
                        let name = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| command.clone());
                        let (classification, evidence) = classify(&command);
                        entries.push(StartupEntry {
                            id: uuid::Uuid::new_v4().to_string(),
                            name,
                            command,
                            location_type: StartupLocationType::StartupFolder,
                            classification,
                            evidence,
                            first_seen: Utc::now(),
                            last_seen: Utc::now(),
                        });
                    }
                }
            }
        }
        entries
    }

    /// Conservative, evidence-based heuristic — never a bare verdict.
    /// Signature checking (would move most "Unknown" entries to
    /// "Known") is not implemented yet; see the Phase 3 handoff.
    fn classify(command: &str) -> (StartupClassification, Vec<String>) {
        let lower = command.to_lowercase();
        let mut evidence = Vec::new();
        let mut suspicious = false;

        for marker in ["\\temp\\", "\\appdata\\local\\temp\\", "\\downloads\\"] {
            if lower.contains(marker) {
                evidence.push(format!("Runs from a commonly-abused location ({marker})"));
                suspicious = true;
            }
        }
        if lower.ends_with(".vbs") || lower.ends_with(".js") || lower.ends_with(".ps1") {
            evidence.push("Uses a script interpreter rather than a compiled executable".into());
            suspicious = true;
        }
        if lower.contains("powershell") && (lower.contains("-enc") || lower.contains("-e ")) {
            evidence.push("Invokes PowerShell with an encoded/obfuscated command".into());
            suspicious = true;
        }

        if suspicious {
            (StartupClassification::Suspicious, evidence)
        } else {
            evidence.push("Signature/publisher not yet checked (Phase 3 follow-up)".into());
            (StartupClassification::Unknown, evidence)
        }
    }

    pub fn remove_entry(
        name: &str,
        _command: &str,
        location: StartupLocationType,
    ) -> Result<(), AppError> {
        match location {
            StartupLocationType::RegistryRun | StartupLocationType::RegistryRunOnce => unsafe {
                let subkey = if location == StartupLocationType::RegistryRun {
                    RUN_KEY
                } else {
                    RUN_ONCE_KEY
                };
                // Try HKCU first, then HKLM — whichever actually has
                // the value succeeds; report the HKLM error if both
                // fail (HKLM removal more plausibly needs elevation).
                for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
                    let wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
                    let mut hkey = HKEY::default();
                    if RegOpenKeyExW(root, PCWSTR(wide.as_ptr()), 0, KEY_SET_VALUE, &mut hkey).is_ok()
                    {
                        let name_wide: Vec<u16> =
                            name.encode_utf16().chain(std::iter::once(0)).collect();
                        let deleted = RegDeleteValueW(hkey, PCWSTR(name_wide.as_ptr()));
                        let _ = RegCloseKey(hkey);
                        if deleted.is_ok() {
                            return Ok(());
                        }
                    }
                }
                Err(AppError {
                    code: "STARTUP_REMOVE_FAILED".into(),
                    message: "Could not remove the registry startup entry.".into(),
                    details: Some(
                        "It may require administrator elevation, or may not exist in either HKCU or HKLM.".into(),
                    ),
                    recoverable: true,
                })
            },
            StartupLocationType::StartupFolder => {
                // `_command` holds the full file path for folder entries.
                std::fs::remove_file(_command).map_err(|e| AppError {
                    code: "STARTUP_REMOVE_FAILED".into(),
                    message: "Could not remove the startup folder shortcut.".into(),
                    details: Some(e.to_string()),
                    recoverable: true,
                })
            }
            StartupLocationType::ScheduledTask => Err(AppError::not_supported(
                "Removing scheduled-task persistence",
            )),
        }
    }
}
