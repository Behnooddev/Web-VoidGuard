use serde::{Deserialize, Serialize};

use crate::models::PortDirection;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum FirewallProtocol {
    Tcp,
    Udp,
    /// Any protocol (`NET_FW_IP_PROTOCOL_ANY` = 256 at the COM layer).
    Any,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum FirewallAction {
    Allow,
    Block,
}

/// One Windows Firewall rule VoidGuard created and is tracking. Unlike
/// the single-port quick actions in `commands::ports` (which only ever
/// toggle one allow rule per port), these are full rules with their own
/// name, scope, and enabled state — mirrored into the local
/// `firewall_rules` table so the list/enable/delete commands don't have
/// to enumerate the *entire* system rule set (hundreds of built-in
/// Windows/app rules) just to find the handful VoidGuard manages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallRule {
    /// The rule's name in Windows Firewall. Names are unique per
    /// direction+profile in the COM API, so this also doubles as the id
    /// used by `Item()`/`Remove()` — see commands::firewall.
    pub name: String,
    pub description: Option<String>,
    pub protocol: FirewallProtocol,
    pub direction: PortDirection,
    pub action: FirewallAction,
    /// Comma-separated local ports/ranges (e.g. `"80,443,8000-8010"`),
    /// or `None` for "any port".
    pub local_ports: Option<String>,
    pub remote_ports: Option<String>,
    /// Comma-separated remote addresses/CIDR ranges, or `None` for "any".
    pub remote_addresses: Option<String>,
    /// Restrict the rule to one application's executable, or `None` to
    /// apply it regardless of which process is involved.
    pub application_path: Option<String>,
    pub enabled: bool,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

/// Typed request to create a new firewall rule. Every field is
/// validated server-side before it ever reaches the COM API — see
/// `commands::firewall::create_firewall_rule`.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateFirewallRuleRequest {
    pub name: String,
    pub description: Option<String>,
    pub protocol: FirewallProtocol,
    pub direction: PortDirection,
    pub action: FirewallAction,
    pub local_ports: Option<String>,
    pub remote_ports: Option<String>,
    pub remote_addresses: Option<String>,
    pub application_path: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetFirewallRuleEnabledRequest {
    pub name: String,
    pub enabled: bool,
}
