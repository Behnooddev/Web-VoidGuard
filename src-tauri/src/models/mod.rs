use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod firewall;
pub mod ports;
pub mod monitoring;
pub mod system_control;
pub use firewall::*;
pub use ports::*;
pub use monitoring::*;
pub use system_control::*;

/// Severity level shared across events, alerts, and detection rules.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "UPPERCASE")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// The category of a monitoring event. Extended incrementally as each
/// phase (process/network/firewall/file/startup/...) comes online.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventCategory {
    FileCreated,
    FileDeleted,
    FileModified,
    ProcessStarted,
    ProcessStopped,
    ServiceChanged,
    PortOpened,
    PortClosed,
    DnsChanged,
    FirewallChanged,
    NetworkChanged,
    StartupChanged,
    SecuritySettingChanged,
    SystemStarted,
}

/// A single normalized system event, persisted to the `events` table
/// and streamed to the UI in real time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub category: EventCategory,
    pub severity: Severity,
    pub source: String,
    pub description: String,
    pub target: Option<String>,
    pub previous_state: Option<String>,
    pub new_state: Option<String>,
    pub related_process: Option<String>,
    pub related_file: Option<String>,
    pub risk_score: u8,
}

/// An audit entry for any privileged / administrative action the
/// application itself performed. Append-only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub user: String,
    pub action: String,
    pub target: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub result: AuditResult,
    pub source: String,
    pub app_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AuditResult {
    Success,
    Failure,
    Denied,
}

/// Point-in-time system resource snapshot used to drive the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: DateTime<Utc>,
    pub cpu_usage_percent: f32,
    pub ram_used_bytes: u64,
    pub ram_total_bytes: u64,
    pub disks: Vec<DiskMetrics>,
    pub process_count: usize,
    pub uptime_seconds: u64,
    pub host_name: Option<String>,
    pub os_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskMetrics {
    pub mount_point: String,
    pub used_bytes: u64,
    pub total_bytes: u64,
}

/// Structured, non-fake error returned to the frontend for any
/// failed / unsupported operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
    pub recoverable: bool,
}

impl AppError {
    pub fn not_supported(feature: &str) -> Self {
        Self {
            code: "NOT_SUPPORTED".into(),
            message: format!("{feature} is not supported on this system/configuration."),
            details: None,
            recoverable: false,
        }
    }
}

/// The aggregate security score shown on the dashboard, with the
/// concrete reasons that produced it (never a bare number).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScore {
    pub score: u8,
    pub reasons: Vec<ScoreReason>,
    pub calculated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreReason {
    pub label: String,
    pub impact: i16,
    pub severity: Severity,
}
