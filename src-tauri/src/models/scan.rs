use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScanType {
    Quick,
    System,
    Network,
    Startup,
    Integrity,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScanStatus {
    Running,
    Completed,
    Failed,
}

/// What a scan actually did and found — never a bare "100%" bar with
/// nothing behind it. Each `ScanStep` corresponds to one real unit of
/// work (an actual enumeration call), emitted live via the
/// `scan-progress` Tauri event as it happens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub scan_id: String,
    pub step_label: String,
    pub step_index: u32,
    pub total_steps: u32,
    pub findings_so_far: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanFinding {
    pub label: String,
    pub detail: String,
    pub severity: crate::models::Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub id: String,
    pub scan_type: ScanType,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub status: ScanStatus,
    pub findings: Vec<ScanFinding>,
    pub summary: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RunScanRequest {
    pub scan_type: ScanType,
    /// Only used when `scan_type` is `Custom` — which of the
    /// individual scan steps to include. Ignored otherwise.
    pub custom_steps: Option<Vec<String>>,
}

/// User-configurable notification preferences, persisted as a single
/// JSON blob under `settings.key = 'notification_settings'`. The
/// frontend polls recent events and decides whether to raise an OS
/// notification against these preferences — see
/// `commands::notifications` and `src/components/NotificationManager.tsx`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub enabled: bool,
    pub min_severity: crate::models::Severity,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            min_severity: crate::models::Severity::High,
        }
    }
}

/// User-configurable retention window (in days) for the high-volume
/// tables — `events`, `process_snapshots`, `port_snapshots` — that
/// otherwise grow without bound. Persisted the same way as
/// `NotificationSettings`, under `settings.key = 'retention_settings'`.
/// `0` means "keep forever" for a given table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionSettings {
    pub events_days: u32,
    pub process_snapshots_days: u32,
    pub port_snapshots_days: u32,
}

impl Default for RetentionSettings {
    fn default() -> Self {
        Self {
            events_days: 30,
            process_snapshots_days: 7,
            port_snapshots_days: 7,
        }
    }
}
