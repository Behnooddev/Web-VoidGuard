pub mod audit;
pub mod events;
pub mod files;
pub mod network;
pub mod ports;
pub mod process;
pub mod risk;
pub mod services;
pub mod startup;
pub mod system;

// firewall.rs (full rule management beyond port open/close) — Phase 4
// Phase 4+ will add: firewall.rs, dns.rs
// Phase 5+ will add: scan.rs, security_score.rs, notifications.rs
//
// Each new module follows the same shape as events.rs/audit.rs:
// a typed request struct in `models`, a #[tauri::command] handler
// here that validates the request, calls the Windows adapter, and
// records the result via commands::audit::record_audit.
