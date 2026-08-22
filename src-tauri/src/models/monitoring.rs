use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FileChangeType {
    Created,
    Modified,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub path: String,
    pub change_type: FileChangeType,
    pub sha256: Option<String>,
    pub size_bytes: Option<u64>,
}

/// A single filesystem location VoidGuard watches. User-configurable
/// in addition to a small built-in default set of security-sensitive
/// paths — see `commands::files::default_watch_scopes`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchScope {
    pub path: String,
    pub recursive: bool,
    pub label: String,
    pub built_in: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum StartupClassification {
    Known,
    Unknown,
    Suspicious,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StartupLocationType {
    RegistryRun,
    RegistryRunOnce,
    StartupFolder,
    ScheduledTask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupEntry {
    pub id: String,
    pub name: String,
    pub command: String,
    pub location_type: StartupLocationType,
    pub classification: StartupClassification,
    /// Plain-language reasons behind the classification — evidence,
    /// never a bare verdict. See SECURITY.md's detection philosophy.
    pub evidence: Vec<String>,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

/// One correlated risk finding produced by combining multiple signals
/// (e.g. unsigned executable + new startup entry + active network
/// connection). Confidence and evidence are always shown together —
/// never a bare score. See SECURITY.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFinding {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub title: String,
    pub description: String,
    pub severity: crate::models::Severity,
    pub confidence: Confidence,
    pub evidence: Vec<String>,
    pub remediation: Option<String>,
    pub related_event_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "UPPERCASE")]
pub enum Confidence {
    Low,
    Medium,
    High,
}
