use crate::commands::audit::record_audit;
use crate::commands::events::record_event;
use crate::db::Db;
use crate::models::{AppError, AuditResult, DnsMode, DnsSettingsRequest, EventCategory, Severity};

/// Applies a per-interface DNS configuration (static servers, or a
/// switch back to DHCP-assigned DNS) via the native `SetInterfaceDnsSettings`
/// API — never `netsh` or any shell invocation, per SECURITY.md.
#[tauri::command]
pub fn change_dns(db: tauri::State<Db>, req: DnsSettingsRequest) -> Result<(), AppError> {
    if let DnsMode::Static = req.mode {
        let Some(primary) = req.primary_dns.as_deref().filter(|s| !s.is_empty()) else {
            return Err(validation_error(
                "A primary DNS server is required for a static configuration.",
            ));
        };
        if !is_valid_ipv4(primary) {
            return Err(validation_error(&format!(
                "'{primary}' is not a valid IPv4 address."
            )));
        }
        if let Some(secondary) = req.secondary_dns.as_deref().filter(|s| !s.is_empty()) {
            if !is_valid_ipv4(secondary) {
                return Err(validation_error(&format!(
                    "'{secondary}' is not a valid IPv4 address."
                )));
            }
        }
    }

    #[cfg(windows)]
    let result = windows_impl::set_dns(&req);
    #[cfg(not(windows))]
    let result: Result<(), AppError> = Err(AppError::not_supported("DNS configuration"));

    let target = format!("adapter {}", req.adapter_id);
    let after = match req.mode {
        DnsMode::Dhcp => "DHCP (automatic)".to_string(),
        DnsMode::Static => {
            let mut servers = vec![req.primary_dns.clone().unwrap_or_default()];
            if let Some(s) = req.secondary_dns.clone().filter(|s| !s.is_empty()) {
                servers.push(s);
            }
            servers.join(", ")
        }
    };

    let _ = record_audit(
        &db,
        "CHANGE_DNS",
        &target,
        None,
        Some(after.clone()),
        match &result {
            Ok(_) => AuditResult::Success,
            Err(_) => AuditResult::Failure,
        },
        "dns",
    );

    if result.is_ok() {
        let _ = record_event(
            &db,
            EventCategory::DnsChanged,
            Severity::Medium,
            "dns",
            &format!("DNS for {target} changed to {after}"),
            Some(target),
        );
    }

    result
}

fn validation_error(message: &str) -> AppError {
    AppError {
        code: "DNS_VALIDATION_FAILED".into(),
        message: message.into(),
        details: None,
        recoverable: false,
    }
}

/// Deliberately strict (rejects leading zeros like "01") to match how
/// Windows itself parses IPv4 literals, and to avoid ambiguous input
/// reaching a native API.
fn is_valid_ipv4(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 4
        && parts
            .iter()
            .all(|p| !p.is_empty() && (p.len() == 1 || !p.starts_with('0')) && p.parse::<u8>().is_ok())
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use windows::core::{GUID, PWSTR};
    use windows::Win32::Foundation::NO_ERROR;
    use windows::Win32::NetworkManagement::IpHelper::{
        SetInterfaceDnsSettings, DNS_INTERFACE_SETTINGS, DNS_INTERFACE_SETTINGS_VERSION1,
    };

    pub fn set_dns(req: &DnsSettingsRequest) -> Result<(), AppError> {
        let guid = parse_guid(&req.adapter_id)?;

        // An empty NameServer list tells Windows to fall back to the
        // DHCP-assigned (or link-local) servers for this interface —
        // verify this against real DHCP/static toggling behavior during
        // the Windows compile pass (see phase 4 handoff checklist).
        let name_server = match req.mode {
            DnsMode::Dhcp => String::new(),
            DnsMode::Static => {
                let mut servers = vec![req.primary_dns.clone().unwrap_or_default()];
                if let Some(s) = req.secondary_dns.clone().filter(|s| !s.is_empty()) {
                    servers.push(s);
                }
                servers.join(",")
            }
        };

        // Kept alive for the duration of the call — DNS_INTERFACE_SETTINGS
        // only borrows the pointer, it doesn't take ownership.
        let mut wide: Vec<u16> = name_server.encode_utf16().chain(std::iter::once(0)).collect();

        let settings = DNS_INTERFACE_SETTINGS {
            Version: DNS_INTERFACE_SETTINGS_VERSION1,
            Flags: 0,
            Domain: PWSTR::null(),
            NameServer: PWSTR(wide.as_mut_ptr()),
            SearchList: PWSTR::null(),
            RegistrationEnabled: 0,
            RegisterAdapterName: 0,
            EnableLLMNR: 0,
            QueryAdapterName: 0,
            ProfileNameServer: PWSTR::null(),
        };

        let result = unsafe { SetInterfaceDnsSettings(guid, &settings) };
        if result != NO_ERROR {
            return Err(AppError {
                code: "DNS_SET_FAILED".into(),
                message: "Windows rejected the DNS configuration change.".into(),
                details: Some(format!("Win32 error code {}", result.0)),
                recoverable: true,
            });
        }
        Ok(())
    }

    /// `NetworkAdapter::adapter_id` is the ANSI GUID string Windows
    /// itself hands back from `GetAdaptersAddresses` (e.g.
    /// `{4D36E972-E325-11CE-BFC1-08002BE10318}`) — parsed by hand here
    /// rather than pulled in as a crate dependency, since it's a fixed,
    /// well-known format.
    fn parse_guid(s: &str) -> Result<GUID, AppError> {
        let bad = || AppError {
            code: "DNS_VALIDATION_FAILED".into(),
            message: "Invalid network adapter reference.".into(),
            details: None,
            recoverable: false,
        };

        let trimmed = s.trim().trim_start_matches('{').trim_end_matches('}');
        let parts: Vec<&str> = trimmed.split('-').collect();
        if parts.len() != 5 {
            return Err(bad());
        }

        let data1 = u32::from_str_radix(parts[0], 16).map_err(|_| bad())?;
        let data2 = u16::from_str_radix(parts[1], 16).map_err(|_| bad())?;
        let data3 = u16::from_str_radix(parts[2], 16).map_err(|_| bad())?;
        let data4_hi = u16::from_str_radix(parts[3], 16).map_err(|_| bad())?;
        let data4_lo = u64::from_str_radix(parts[4], 16).map_err(|_| bad())?;

        let mut data4 = [0u8; 8];
        data4[0] = (data4_hi >> 8) as u8;
        data4[1] = (data4_hi & 0xFF) as u8;
        for i in 0..6 {
            data4[2 + i] = ((data4_lo >> (8 * (5 - i))) & 0xFF) as u8;
        }

        Ok(GUID {
            data1,
            data2,
            data3,
            data4,
        })
    }
}
