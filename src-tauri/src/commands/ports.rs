use crate::commands::audit::record_audit;
use crate::commands::events::record_event;
use crate::db::Db;
use crate::models::{
    AppError, AuditResult, EventCategory, ListeningPort, PortDirection, PortProtocol,
    PortRisk, PortRuleRequest, Severity,
};

/// Cross-platform entry point used by the Tauri command. The actual
/// enumeration only exists on Windows (`GetExtendedTcpTable` /
/// `GetExtendedUdpTable`); everywhere else this returns a structured
/// "not supported" error rather than fake data, per the project's
/// implementation rule.
#[tauri::command]
pub fn list_listening_ports() -> Result<Vec<ListeningPort>, AppError> {
    #[cfg(windows)]
    {
        windows_impl::enumerate_ports()
    }
    #[cfg(not(windows))]
    {
        Err(AppError::not_supported("Open port enumeration"))
    }
}

/// Terminates the process that owns a given listening port. This is a
/// thin, explicit wrapper around `process::terminate_process` — kept
/// as its own command so the audit trail records it as a **port**
/// action ("terminated the process holding port 4444") rather than a
/// generic process kill, and so the UI's Ports page doesn't need to
/// reach into the Processes module to do this.
#[tauri::command]
pub fn terminate_port_owner(
    db: tauri::State<Db>,
    sys_handle: tauri::State<crate::commands::system::SysHandle>,
    port: u16,
    pid: u32,
) -> Result<(), AppError> {
    let result = crate::commands::process::terminate_process(sys_handle, pid);

    let target = format!("port {port} (pid {pid})");
    let _ = record_audit(
        &db,
        "TERMINATE_PORT_OWNER",
        &target,
        None,
        None,
        match &result {
            Ok(_) => AuditResult::Success,
            Err(_) => AuditResult::Failure,
        },
        "ports",
    );

    if result.is_ok() {
        let _ = record_event(
            &db,
            EventCategory::PortClosed,
            Severity::Medium,
            "ports",
            &format!("Process owning port {port} was terminated by the user"),
            Some(target),
        );
    }

    result
}

/// Opens (allows) a port through Windows Firewall by creating a
/// named inbound/outbound rule via the native firewall COM API
/// (`INetFwPolicy2` / `INetFwRule`) — never `netsh` or any shell
/// invocation. Always confirmed in the UI before this is called, and
/// always audited here regardless of outcome.
#[tauri::command]
pub fn open_port(db: tauri::State<Db>, req: PortRuleRequest) -> Result<(), AppError> {
    #[cfg(windows)]
    {
        let result = windows_impl::set_port_rule(&req, true);
        audit_port_rule(&db, "OPEN_PORT", &req, &result);
        result
    }
    #[cfg(not(windows))]
    {
        let result = Err(AppError::not_supported("Firewall port control"));
        audit_port_rule(&db, "OPEN_PORT", &req, &result);
        result
    }
}

/// Closes (blocks/removes the allow rule for) a port through Windows
/// Firewall. Same COM-API-only, always-audited approach as `open_port`.
#[tauri::command]
pub fn close_port(db: tauri::State<Db>, req: PortRuleRequest) -> Result<(), AppError> {
    #[cfg(windows)]
    {
        let result = windows_impl::set_port_rule(&req, false);
        audit_port_rule(&db, "CLOSE_PORT", &req, &result);
        result
    }
    #[cfg(not(windows))]
    {
        let result = Err(AppError::not_supported("Firewall port control"));
        audit_port_rule(&db, "CLOSE_PORT", &req, &result);
        result
    }
}

fn audit_port_rule(db: &Db, action: &str, req: &PortRuleRequest, result: &Result<(), AppError>) {
    let target = format!("{:?} port {} ({:?})", req.protocol, req.port, req.direction);
    let _ = record_audit(
        db,
        action,
        &target,
        None,
        None,
        match result {
            Ok(_) => AuditResult::Success,
            Err(_) => AuditResult::Failure,
        },
        "ports",
    );
    if result.is_ok() {
        let _ = record_event(
            db,
            EventCategory::FirewallChanged,
            Severity::Medium,
            "ports",
            &format!("{action} for {target}"),
            Some(target),
        );
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::mem::size_of;
    use windows::core::{HSTRING, PWSTR};
    use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, NO_ERROR};
    use windows::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID,
        MIB_UDPROW_OWNER_PID, MIB_UDPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_ALL,
        UDP_TABLE_OWNER_PID,
    };
    use windows::Win32::Networking::WinSock::AF_INET;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::NetworkManagement::WindowsFirewall::{
        INetFwPolicy2, INetFwRule, NetFwPolicy2, NetFwRule, NET_FW_ACTION_ALLOW,
        NET_FW_PROFILE2_ALL, NET_FW_RULE_DIR_IN, NET_FW_RULE_DIR_OUT,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::Foundation::CloseHandle;

    /// Reads the full TCP + UDP owner-PID tables and turns them into
    /// `ListeningPort` rows, resolving each PID's image name/path
    /// where the process is still running and accessible.
    pub fn enumerate_ports() -> Result<Vec<ListeningPort>, AppError> {
        let mut ports = enumerate_tcp()?;
        ports.extend(enumerate_udp()?);
        Ok(ports)
    }

    fn enumerate_tcp() -> Result<Vec<ListeningPort>, AppError> {
        unsafe {
            let mut size: u32 = 0;
            // First call with a null buffer to learn the required size.
            let _ = GetExtendedTcpTable(
                None,
                &mut size,
                false,
                AF_INET.0 as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );

            let mut buffer = vec![0u8; size as usize];
            let result = GetExtendedTcpTable(
                Some(buffer.as_mut_ptr() as *mut _),
                &mut size,
                false,
                AF_INET.0 as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
            if result != NO_ERROR.0 {
                return Err(table_error("TCP", result));
            }

            let table = &*(buffer.as_ptr() as *const MIB_TCPTABLE_OWNER_PID);
            let count = table.dwNumEntries as usize;
            let rows = std::slice::from_raw_parts(table.table.as_ptr(), count);

            Ok(rows
                .iter()
                .filter(|r| is_listening_tcp(r))
                .map(|r| {
                    let pid = r.dwOwningPid;
                    let (name, exe) = resolve_process(pid);
                    ListeningPort {
                        protocol: PortProtocol::Tcp,
                        local_address: format_ipv4(r.dwLocalAddr),
                        port: u16::from_be((r.dwLocalPort & 0xFFFF) as u16),
                        pid: Some(pid),
                        process_name: name,
                        executable_path: exe,
                        status: "LISTENING".into(),
                        risk: classify_risk(u16::from_be((r.dwLocalPort & 0xFFFF) as u16)),
                        firewall_allowed: None,
                    }
                })
                .collect())
        }
    }

    fn enumerate_udp() -> Result<Vec<ListeningPort>, AppError> {
        unsafe {
            let mut size: u32 = 0;
            let _ = GetExtendedUdpTable(
                None,
                &mut size,
                false,
                AF_INET.0 as u32,
                UDP_TABLE_OWNER_PID,
                0,
            );

            let mut buffer = vec![0u8; size as usize];
            let result = GetExtendedUdpTable(
                Some(buffer.as_mut_ptr() as *mut _),
                &mut size,
                false,
                AF_INET.0 as u32,
                UDP_TABLE_OWNER_PID,
                0,
            );
            if result != NO_ERROR.0 {
                return Err(table_error("UDP", result));
            }

            let table = &*(buffer.as_ptr() as *const MIB_UDPTABLE_OWNER_PID);
            let count = table.dwNumEntries as usize;
            let rows = std::slice::from_raw_parts(table.table.as_ptr(), count);

            Ok(rows
                .iter()
                .map(|r: &MIB_UDPROW_OWNER_PID| {
                    let pid = r.dwOwningPid;
                    let (name, exe) = resolve_process(pid);
                    let port = u16::from_be((r.dwLocalPort & 0xFFFF) as u16);
                    ListeningPort {
                        protocol: PortProtocol::Udp,
                        local_address: format_ipv4(r.dwLocalAddr),
                        port,
                        pid: Some(pid),
                        process_name: name,
                        executable_path: exe,
                        status: "BOUND".into(),
                        risk: classify_risk(port),
                        firewall_allowed: None,
                    }
                })
                .collect())
        }
    }

    fn is_listening_tcp(row: &MIB_TCPROW_OWNER_PID) -> bool {
        // MIB_TCP_STATE_LISTEN == 2
        row.dwState == 2
    }

    fn format_ipv4(addr: u32) -> String {
        let bytes = addr.to_le_bytes();
        format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
    }

    /// Unknown/rarely-used high ports get flagged Medium by default so
    /// the UI has *some* signal; well-known infrastructure ports are
    /// Low. This is intentionally conservative — see SECURITY.md on
    /// not auto-labeling things as malicious.
    fn classify_risk(port: u16) -> PortRisk {
        match port {
            80 | 443 | 53 | 445 | 135 | 3389 => PortRisk::Low,
            0..=1023 => PortRisk::Low,
            _ => PortRisk::Medium,
        }
    }

    fn table_error(proto: &str, win32_err: u32) -> AppError {
        AppError {
            code: format!("{proto}_TABLE_READ_FAILED"),
            message: format!("Failed to read the {proto} connection table from Windows."),
            details: Some(format!("Win32 error code {win32_err}")),
            recoverable: true,
        }
    }

    fn resolve_process(pid: u32) -> (Option<String>, Option<String>) {
        if pid == 0 {
            return (Some("System Idle".into()), None);
        }
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
            let Ok(handle) = handle else {
                return (None, None);
            };
            let mut buf = [0u16; 260];
            let mut len: u32 = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(buf.as_mut_ptr()),
                &mut len,
            );
            let _ = CloseHandle(handle);
            if ok.is_ok() {
                let path = String::from_utf16_lossy(&buf[..len as usize]);
                let name = path
                    .rsplit(['\\', '/'])
                    .next()
                    .unwrap_or(&path)
                    .to_string();
                (Some(name), Some(path))
            } else {
                (None, None)
            }
        }
    }

    /// Creates or removes a single, narrowly-scoped Windows Firewall
    /// rule for exactly one port/protocol/direction via COM
    /// (`HNetCfg.FwPolicy2`). This is the same mechanism the Windows
    /// Firewall control panel itself uses — no shell, no `netsh`.
    pub fn set_port_rule(req: &PortRuleRequest, allow: bool) -> Result<(), AppError> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

            let policy: INetFwPolicy2 = CoCreateInstance(&NetFwPolicy2, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| com_error("Could not access Windows Firewall.", e))?;

            let rule_name = format!(
                "VoidGuard - {:?} {} ({:?})",
                req.protocol, req.port, req.direction
            );

            if !allow {
                // Best-effort removal; a missing rule is not an error.
                let rules = policy
                    .Rules()
                    .map_err(|e| com_error("Could not read firewall rules.", e))?;
                let _ = rules.Remove(&HSTRING::from(rule_name.as_str()));
                return Ok(());
            }

            let rule: INetFwRule = CoCreateInstance(&NetFwRule, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| com_error("Could not create a firewall rule object.", e))?;

            rule.SetName(&HSTRING::from(rule_name.as_str()))
                .map_err(|e| com_error("Could not name the firewall rule.", e))?;
            rule.SetDescription(&HSTRING::from(
                "Created by VoidGuard's port control panel.",
            ))
            .ok();
            rule.SetProtocol(match req.protocol {
                PortProtocol::Tcp => 6,  // IPPROTO_TCP
                PortProtocol::Udp => 17, // IPPROTO_UDP
            })
            .map_err(|e| com_error("Could not set the rule protocol.", e))?;
            rule.SetLocalPorts(&HSTRING::from(req.port.to_string().as_str()))
                .map_err(|e| com_error("Could not set the rule port.", e))?;
            rule.SetDirection(match req.direction {
                PortDirection::Inbound => NET_FW_RULE_DIR_IN,
                PortDirection::Outbound => NET_FW_RULE_DIR_OUT,
            })
            .map_err(|e| com_error("Could not set the rule direction.", e))?;
            rule.SetAction(NET_FW_ACTION_ALLOW)
                .map_err(|e| com_error("Could not set the rule action.", e))?;
            rule.SetEnabled(true)
                .map_err(|e| com_error("Could not enable the rule.", e))?;
            rule.SetProfiles(NET_FW_PROFILE2_ALL.0)
                .map_err(|e| com_error("Could not set the rule profile scope.", e))?;

            let rules = policy
                .Rules()
                .map_err(|e| com_error("Could not read firewall rules.", e))?;
            rules
                .Add(&rule)
                .map_err(|e| com_error("Windows rejected the new firewall rule.", e))?;

            Ok(())
        }
    }

    fn com_error(message: &str, e: windows::core::Error) -> AppError {
        AppError {
            code: "FIREWALL_COM_ERROR".into(),
            message: message.into(),
            details: Some(e.message().to_string()),
            recoverable: true,
        }
    }

    // Keeps the `size_of` import used above from triggering an unused
    // warning if the buffer-sizing approach changes in a later pass.
    #[allow(dead_code)]
    fn _unused() -> usize {
        size_of::<MIB_TCPTABLE_OWNER_PID>()
    }
}
