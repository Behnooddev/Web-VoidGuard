use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

/// Shared, mutex-guarded SQLite connection handed to Tauri as managed state.
/// Phase 1 only creates the schema; later phases add repository
/// modules (events_repo, audit_repo, process_repo, ...) that operate
/// on this connection rather than opening their own.
pub struct Db(pub Mutex<Connection>);

pub fn app_data_dir() -> PathBuf {
    // On Windows this resolves to %APPDATA%/VoidGuard via Tauri's path
    // resolver at runtime; here we fall back to a local dir for
    // non-Windows dev/testing.
    let base = dirs_next::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("VoidGuard")
}

pub fn init() -> Result<Db> {
    let dir = app_data_dir();
    std::fs::create_dir_all(&dir)?;
    let db_path = dir.join("voidguard.db");
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    run_migrations(&conn)?;
    Ok(Db(Mutex::new(conn)))
}

/// Forward-only, additive migrations tracked by `schema_version`.
/// Each phase of the app adds its own tables here rather than
/// rewriting earlier ones, so upgrades never lose history.
fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        );

        CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            category TEXT NOT NULL,
            severity TEXT NOT NULL,
            source TEXT NOT NULL,
            description TEXT NOT NULL,
            target TEXT,
            previous_state TEXT,
            new_state TEXT,
            related_process TEXT,
            related_file TEXT,
            risk_score INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp DESC);
        CREATE INDEX IF NOT EXISTS idx_events_category ON events(category);

        CREATE TABLE IF NOT EXISTS alerts (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            rule_id TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT NOT NULL,
            severity TEXT NOT NULL,
            confidence TEXT NOT NULL,
            evidence TEXT NOT NULL,
            remediation TEXT,
            acknowledged INTEGER NOT NULL DEFAULT 0,
            related_event_id TEXT REFERENCES events(id)
        );

        -- Append-only audit trail. No UPDATE/DELETE statements are
        -- ever issued against this table by application code.
        CREATE TABLE IF NOT EXISTS audit_logs (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            user TEXT NOT NULL,
            action TEXT NOT NULL,
            target TEXT NOT NULL,
            before TEXT,
            after TEXT,
            result TEXT NOT NULL,
            source TEXT NOT NULL,
            app_version TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS process_snapshots (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            pid INTEGER NOT NULL,
            parent_pid INTEGER,
            name TEXT NOT NULL,
            exe_path TEXT,
            cpu_percent REAL,
            memory_bytes INTEGER,
            start_time TEXT,
            publisher TEXT,
            signed INTEGER,
            integrity_level TEXT,
            sha256 TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_proc_snap_ts ON process_snapshots(timestamp DESC);

        CREATE TABLE IF NOT EXISTS network_snapshots (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            adapter_name TEXT NOT NULL,
            ipv4 TEXT,
            ipv6 TEXT,
            gateway TEXT,
            dns TEXT,
            mac_address TEXT,
            state TEXT
        );

        CREATE TABLE IF NOT EXISTS port_snapshots (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            protocol TEXT NOT NULL,
            local_address TEXT NOT NULL,
            port INTEGER NOT NULL,
            pid INTEGER,
            process_name TEXT,
            status TEXT,
            risk TEXT
        );

        CREATE TABLE IF NOT EXISTS services (
            name TEXT PRIMARY KEY,
            display_name TEXT,
            status TEXT,
            startup_type TEXT,
            publisher TEXT,
            executable TEXT,
            description TEXT,
            last_seen TEXT
        );

        -- Phase 4: tracks the firewall rules VoidGuard itself created
        -- (via commands::firewall), not the entire system rule set.
        -- `name` is unique so `create_firewall_rule` can upsert safely
        -- if the same rule name is recreated after being edited.
        CREATE TABLE IF NOT EXISTS firewall_rules (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            protocol TEXT,
            local_port TEXT,
            remote_port TEXT,
            remote_addresses TEXT,
            application TEXT,
            direction TEXT,
            action TEXT,
            enabled INTEGER,
            last_seen TEXT
        );

        CREATE TABLE IF NOT EXISTS file_events (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            path TEXT NOT NULL,
            change_type TEXT NOT NULL,
            sha256 TEXT,
            signer TEXT,
            size_bytes INTEGER,
            created_at TEXT,
            modified_at TEXT
        );

        CREATE TABLE IF NOT EXISTS startup_entries (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            command TEXT NOT NULL,
            location_type TEXT NOT NULL,
            classification TEXT NOT NULL,
            evidence TEXT,
            first_seen TEXT NOT NULL,
            last_seen TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS scan_results (
            id TEXT PRIMARY KEY,
            scan_type TEXT NOT NULL,
            started_at TEXT NOT NULL,
            finished_at TEXT,
            status TEXT NOT NULL,
            findings_count INTEGER NOT NULL DEFAULT 0,
            summary TEXT
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS watch_scopes (
            path TEXT PRIMARY KEY,
            recursive INTEGER NOT NULL DEFAULT 1,
            label TEXT NOT NULL,
            built_in INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS risk_findings (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT NOT NULL,
            severity TEXT NOT NULL,
            confidence TEXT NOT NULL,
            evidence TEXT NOT NULL,
            remediation TEXT,
            related_event_ids TEXT NOT NULL DEFAULT '[]'
        );
        CREATE INDEX IF NOT EXISTS idx_risk_findings_ts ON risk_findings(timestamp DESC);
        "#,
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO schema_version (version) VALUES (1)",
        [],
    )?;

    Ok(())
}
