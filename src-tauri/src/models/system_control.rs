use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAdapter {
    /// The adapter's GUID (e.g. `{4D36E972-...}`), from `GetAdaptersAddresses`'
    /// `AdapterName` field. This — never `name` — is what identifies the
    /// adapter to `commands::dns::change_dns`: friendly names can repeat
    /// or change, the GUID doesn't.
    pub adapter_id: String,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DnsMode {
    Dhcp,
    Static,
}

/// Typed request to change one adapter's DNS configuration. Always
/// re-validated server-side (see `commands::dns::change_dns`) — the
/// frontend's own IPv4 checks are just for fast feedback, never trusted.
#[derive(Debug, Clone, Deserialize)]
pub struct DnsSettingsRequest {
    /// Must be a `NetworkAdapter::adapter_id`, not a display name.
    pub adapter_id: String,
    pub mode: DnsMode,
    /// Required when `mode` is `STATIC`; ignored for `DHCP`.
    pub primary_dns: Option<String>,
    /// Optional even in `STATIC` mode.
    pub secondary_dns: Option<String>,
}
