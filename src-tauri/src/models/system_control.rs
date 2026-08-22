use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAdapter {
    pub name: String,
    pub description: String,
    pub adapter_type: String, // "Ethernet" | "Wi-Fi" | "VPN" | "Loopback" | "Other"
    pub status: String,       // "Up" | "Down" | "Unknown"
    pub mac_address: Option<String>,
    pub ipv4_addresses: Vec<String>,
    pub ipv6_addresses: Vec<String>,
    pub gateway: Option<String>,
    pub dns_servers: Vec<String>,
    pub dhcp_enabled: Option<bool>,
    pub link_speed_mbps: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ServiceStatus {
    Running,
    Stopped,
    Paused,
    StartPending,
    StopPending,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StartupType {
    Automatic,
    AutomaticDelayed,
    Manual,
    Disabled,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub status: ServiceStatus,
    pub startup_type: StartupType,
    pub executable: Option<String>,
    pub description: Option<String>,
    /// Services Windows itself treats as protected (cannot safely be
    /// stopped without risking system stability). The UI must show an
    /// extra-strength confirmation for these — see SECURITY.md.
    pub protected: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ServiceAction {
    Start,
    Stop,
    Restart,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceActionRequest {
    pub service_name: String,
    pub action: ServiceAction,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChangeStartupTypeRequest {
    pub service_name: String,
    pub startup_type: StartupType,
}
