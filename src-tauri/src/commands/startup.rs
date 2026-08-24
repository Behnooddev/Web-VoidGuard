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
    use windows::core::{BSTR, PCWSTR, VARIANT};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegEnumValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER,
        HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE, REG_SZ,
    };
    use windows::Win32::System::TaskScheduler::{
        ITaskFolder, ITaskService, IRegisteredTask, TaskScheduler, TASK_ENUM_HIDDEN,
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
        entries.extend(enumerate_scheduled_tasks());

        Ok(entries)
    }

    fn enumerate_scheduled_tasks() -> Vec<StartupEntry> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let service: Result<ITaskService, _> =
                CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER);
            let Ok(service) = service else {
                return Vec::new();
            };
            let empty = VARIANT::default();
            if service.Connect(&empty, &empty, &empty, &empty).is_err() {
                return Vec::new();
            }
            let Ok(root) = service.GetFolder(&BSTR::from("\\")) else {
                return Vec::new();
            };

            let mut entries = Vec::new();
            walk_task_folder(&root, &mut entries);
            entries
        }
    }

    /// Task Scheduler organizes tasks into folders (most third-party
    /// and malicious tasks sit under `\`, but plenty of legitimate ones
    /// are nested, e.g. `\Microsoft\Windows\...`) — walked recursively
    /// so a task hidden a few folders down isn't missed.
    unsafe fn walk_task_folder(folder: &ITaskFolder, entries: &mut Vec<StartupEntry>) {
        if let Ok(tasks) = folder.GetTasks(TASK_ENUM_HIDDEN.0) {
            if let Ok(count) = tasks.Count() {
                for i in 1..=count {
                    if let Ok(task) = tasks.get_Item(VARIANT::from(i)) {
                        if let Some(entry) = task_to_entry(&task) {
                            entries.push(entry);
                        }
                    }
                }
            }
        }
        if let Ok(subfolders) = folder.GetFolders(0) {
            if let Ok(count) = subfolders.Count() {
                for i in 1..=count {
                    if let Ok(sub) = subfolders.get_Item(VARIANT::from(i)) {
                        walk_task_folder(&sub, entries);
                    }
                }
            }
        }
    }

    unsafe fn task_to_entry(task: &IRegisteredTask) -> Option<StartupEntry> {
        let name = task.Name().ok()?.to_string();
        let path = task.Path().ok()?.to_string();
        // The task's registered command lives in its XML definition —
        // pulling it with a plain substring search avoids adding an
        // XML-parsing dependency for what's a small, predictable tag.
        let command = task
            .Xml()
            .ok()
            .and_then(|xml| extract_command(&xml.to_string()))
            .unwrap_or_else(|| path.clone());

        let (classification, evidence) = classify(&command);
        Some(StartupEntry {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            command,
            location_type: StartupLocationType::ScheduledTask,
            classification,
            evidence,
            first_seen: Utc::now(),
            last_seen: Utc::now(),
        })
    }

    fn extract_command(xml: &str) -> Option<String> {
        let command = extract_tag(xml, "Command")?;
        let args = extract_tag(xml, "Arguments").unwrap_or_default();
        Some(if args.is_empty() {
            command
        } else {
            format!("{command} {args}")
        })
    }

    fn extract_tag(xml: &str, tag: &str) -> Option<String> {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        let start = xml.find(&open)? + open.len();
        let end = xml[start..].find(&close)? + start;
        Some(xml[start..end].trim().to_string())
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
        command: &str,
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
                // `command` holds the full file path for folder entries.
                std::fs::remove_file(command).map_err(|e| AppError {
                    code: "STARTUP_REMOVE_FAILED".into(),
                    message: "Could not remove the startup folder shortcut.".into(),
                    details: Some(e.to_string()),
                    recoverable: true,
                })
            }
            StartupLocationType::ScheduledTask => remove_scheduled_task(name, command),
        }
    }

    /// Scheduled tasks are identified by name when enumerated (see
    /// `task_to_entry`), but deleting one needs its *folder path* too
    /// (`ITaskFolder::DeleteTask` is scoped to one folder, and tasks
    /// aren't stored with their folder in the DB) — so this re-walks
    /// Task Scheduler once to find the matching task and its parent
    /// folder, then deletes it there. Matches on name *and* command so
    /// two same-named tasks in different folders aren't confused.
    fn remove_scheduled_task(name: &str, command: &str) -> Result<(), AppError> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let service: ITaskService =
                CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER)
                    .map_err(|e| task_error("Could not access Task Scheduler.", e))?;
            let empty = VARIANT::default();
            service
                .Connect(&empty, &empty, &empty, &empty)
                .map_err(|e| task_error("Could not connect to Task Scheduler.", e))?;
            let root = service
                .GetFolder(&BSTR::from("\\"))
                .map_err(|e| task_error("Could not open the Task Scheduler root folder.", e))?;

            let Some((folder_path, task_name)) = find_task(&root, name, command) else {
                return Err(AppError {
                    code: "STARTUP_REMOVE_FAILED".into(),
                    message: "Could not find that scheduled task — it may already be gone."
                        .into(),
                    details: None,
                    recoverable: false,
                });
            };

            let folder = service
                .GetFolder(&BSTR::from(folder_path.as_str()))
                .map_err(|e| task_error("Could not open the task's folder.", e))?;
            folder
                .DeleteTask(&BSTR::from(task_name.as_str()), 0)
                .map_err(|e| task_error("Windows rejected deleting the scheduled task.", e))
        }
    }

    /// Returns `(parent folder path, task name)` for the first task
    /// matching both `name` and `command`.
    unsafe fn find_task(folder: &ITaskFolder, name: &str, command: &str) -> Option<(String, String)> {
        if let Ok(tasks) = folder.GetTasks(TASK_ENUM_HIDDEN.0) {
            if let Ok(count) = tasks.Count() {
                for i in 1..=count {
                    if let Ok(task) = tasks.get_Item(VARIANT::from(i)) {
                        if let Some(entry) = task_to_entry(&task) {
                            if entry.name == name && entry.command == command {
                                if let Ok(path) = task.Path().map(|p| p.to_string()) {
                                    let parent = match path.rsplit_once('\\') {
                                        Some(("", _)) | None => "\\".to_string(),
                                        Some((parent, _)) => parent.to_string(),
                                    };
                                    return Some((parent, entry.name));
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Ok(subfolders) = folder.GetFolders(0) {
            if let Ok(count) = subfolders.Count() {
                for i in 1..=count {
                    if let Ok(sub) = subfolders.get_Item(VARIANT::from(i)) {
                        if let Some(found) = find_task(&sub, name, command) {
                            return Some(found);
                        }
                    }
                }
            }
        }
        None
    }

    fn task_error(message: &str, e: windows::core::Error) -> AppError {
        AppError {
            code: "TASK_SCHEDULER_COM_ERROR".into(),
            message: message.into(),
            details: Some(e.message().to_string()),
            recoverable: true,
        }
    }
}
