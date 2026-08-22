use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PortProtocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PortRisk {
    Low,
    Medium,
    High,
    Unknown,
}

/// One row of the open-port table: what is listening, where, and
/// owned by which process. Populated from `GetExtendedTcpTable` /
/// `GetExtendedUdpTable` on Windows (see commands::ports).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListeningPort {
    pub protocol: PortProtocol,
    pub local_address: String,
    pub port: u16,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    pub executable_path: Option<String>,
    pub status: String,
    pub risk: PortRisk,
    /// True if a Windows Firewall rule already exists for this exact
    /// port, so the UI can show "Open"/"Blocked" state accurately
    /// instead of guessing.
    pub firewall_allowed: Option<bool>,
}

/// Typed request to allow or block a specific port through Windows
/// Firewall. Never a raw rule string — direction/action/port are all
/// separately validated fields.
#[derive(Debug, Clone, Deserialize)]
pub struct PortRuleRequest {
    pub port: u16,
    pub protocol: PortProtocol,
    pub direction: PortDirection,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PortDirection {
    Inbound,
    Outbound,
}
