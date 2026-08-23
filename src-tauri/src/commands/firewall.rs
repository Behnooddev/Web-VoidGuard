use chrono::Utc;
use rusqlite::params;

use crate::commands::audit::record_audit;
use crate::commands::events::record_event;
use crate::db::Db;
use crate::models::{
    AppError, AuditResult, CreateFirewallRuleRequest, EventCategory, FirewallAction,
    FirewallProtocol, FirewallRule, PortDirection, SetFirewallRuleEnabledRequest, Severity,
};

/// Lists the firewall rules VoidGuard itself created and is tracking
/// (the local `firewall_rules` table), rather than the entire system
/// rule set — Windows ships hundreds of built-in/app rules that aren't
/// this app's to manage. See `models::firewall::FirewallRule` for why.
#[tauri::command]
pub fn list_firewall_rules(db: tauri::State<Db>) -> Result<Vec<FirewallRule>, AppError> {
    let conn = db.0.lock().map_err(lock_error)?;
    let mut stmt = conn
        .prepare(
            "SELECT name, description, protocol, direction, action, local_port,
                remote_port, remote_addresses, application, enabled, last_seen
             FROM firewall_rules ORDER BY last_seen DESC",
        )
        .map_err(db_error)?;

    let rows = stmt
        .query_map([], |row| {
            let protocol: String = row.get(2)?;
            let direction: String = row.get(3)?;
            let action: String = row.get(4)?;
            let enabled: i64 = row.get(9)?;
            Ok(FirewallRule {
                name: row.get(0)?,
                description: row.get(1)?,
                protocol: serde_json::from_str(&protocol).unwrap_or(FirewallProtocol::Any),
                direction: serde_json::from_str(&direction).unwrap_or(PortDirection::Inbound),
                action: serde_json::from_str(&action).unwrap_or(FirewallAction::Block),
                local_ports: row.get(5)?,
                remote_ports: row.get(6)?,
                remote_addresses: row.get(7)?,
                application_path: row.get(8)?,
                enabled: enabled != 0,
                last_seen: row
                    .get::<_, String>(10)?
                    .parse()
                    .unwrap_or_else(|_| Utc::now()),
            })
        })
        .map_err(db_error)?;

    rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
}

/// Creates a new Windows Firewall rule via COM (`INetFwPolicy2`/`INetFwRule`
/// — the same mechanism the Firewall control panel uses, never `netsh`
/// or a shell), then records it locally so `list_firewall_rules` can
/// find it again without enumerating the whole system rule set.
#[tauri::command]
pub fn create_firewall_rule(
    db: tauri::State<Db>,
    req: CreateFirewallRuleRequest,
) -> Result<(), AppError> {
    validate_rule_name(&req.name)?;

    #[cfg(windows)]
    let result = windows_impl::add_rule(&req);
    #[cfg(not(windows))]
    let result: Result<(), AppError> = Err(AppError::not_supported("Firewall rule management"));

    let target = req.name.clone();
    let _ = record_audit(
        &db,
        "CREATE_FIREWALL_RULE",
        &target,
        None,
        Some(describe_rule(&req)),
        match &result {
            Ok(_) => AuditResult::Success,
            Err(_) => AuditResult::Failure,
        },
        "firewall",
    );

    if result.is_ok() {
        let _ = persist_rule(&db, &req);
        let _ = record_event(
            &db,
            EventCategory::FirewallChanged,
            Severity::Medium,
            "firewall",
            &format!("Firewall rule '{}' created", req.name),
            Some(target),
        );
    }

    result
}

/// Enables/disables an existing rule in place (`INetFwRule::SetEnabled`
/// via `INetFwRules::Item(name)` — a documented, name-keyed lookup, not
/// a full collection enumeration).
#[tauri::command]
pub fn set_firewall_rule_enabled(
    db: tauri::State<Db>,
    req: SetFirewallRuleEnabledRequest,
) -> Result<(), AppError> {
    #[cfg(windows)]
    let result = windows_impl::set_enabled(&req.name, req.enabled);
    #[cfg(not(windows))]
    let result: Result<(), AppError> = Err(AppError::not_supported("Firewall rule management"));

    let _ = record_audit(
        &db,
        "SET_FIREWALL_RULE_ENABLED",
        &req.name,
        None,
        Some(if req.enabled { "ENABLED" } else { "DISABLED" }.to_string()),
        match &result {
            Ok(_) => AuditResult::Success,
            Err(_) => AuditResult::Failure,
        },
        "firewall",
    );

    if result.is_ok() {
        if let Ok(conn) = db.0.lock() {
            let _ = conn.execute(
                "UPDATE firewall_rules SET enabled = ?1 WHERE name = ?2",
                params![req.enabled as i64, req.name],
            );
        }
        let _ = record_event(
            &db,
            EventCategory::FirewallChanged,
            Severity::Low,
            "firewall",
            &format!(
                "Firewall rule '{}' {}",
                req.name,
                if req.enabled { "enabled" } else { "disabled" }
            ),
            Some(req.name.clone()),
        );
    }

    result
}

/// Deletes a rule VoidGuard created. Only ever targets rules by their
/// exact name via `INetFwRules::Remove` — never a bulk/pattern delete.
#[tauri::command]
pub fn delete_firewall_rule(db: tauri::State<Db>, name: String) -> Result<(), AppError> {
    #[cfg(windows)]
    let result = windows_impl::remove_rule(&name);
    #[cfg(not(windows))]
    let result: Result<(), AppError> = Err(AppError::not_supported("Firewall rule management"));

    let _ = record_audit(
        &db,
        "DELETE_FIREWALL_RULE",
        &name,
        None,
        None,
        match &result {
            Ok(_) => AuditResult::Success,
            Err(_) => AuditResult::Failure,
        },
        "firewall",
    );

    if result.is_ok() {
        if let Ok(conn) = db.0.lock() {
            let _ = conn.execute("DELETE FROM firewall_rules WHERE name = ?1", params![name]);
        }
        let _ = record_event(
            &db,
            EventCategory::FirewallChanged,
            Severity::Medium,
            "firewall",
            &format!("Firewall rule '{name}' deleted"),
            Some(name.clone()),
        );
    }

    result
}

fn validate_rule_name(name: &str) -> Result<(), AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.len() > 200 {
        return Err(AppError {
            code: "FIREWALL_VALIDATION_FAILED".into(),
            message: "Rule name must be between 1 and 200 characters.".into(),
            details: None,
            recoverable: false,
        });
    }
    Ok(())
}

fn describe_rule(req: &CreateFirewallRuleRequest) -> String {
    format!(
        "{:?} {:?} {:?} local={} remote={} app={}",
        req.action,
        req.direction,
        req.protocol,
        req.local_ports.clone().unwrap_or_else(|| "any".into()),
        req.remote_addresses.clone().unwrap_or_else(|| "any".into()),
        req.application_path.clone().unwrap_or_else(|| "any".into()),
    )
}

fn persist_rule(db: &Db, req: &CreateFirewallRuleRequest) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO firewall_rules
            (id, name, description, protocol, direction, action, local_port,
             remote_port, remote_addresses, application, enabled, last_seen)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(name) DO UPDATE SET
            description = excluded.description,
            protocol = excluded.protocol,
            direction = excluded.direction,
            action = excluded.action,
            local_port = excluded.local_port,
            remote_port = excluded.remote_port,
            remote_addresses = excluded.remote_addresses,
            application = excluded.application,
            enabled = excluded.enabled,
            last_seen = excluded.last_seen",
        params![
            uuid::Uuid::new_v4().to_string(),
            req.name,
            req.description,
            serde_json::to_string(&req.protocol).unwrap(),
            serde_json::to_string(&req.direction).unwrap(),
            serde_json::to_string(&req.action).unwrap(),
            req.local_ports,
            req.remote_ports,
            req.remote_addresses,
            req.application_path,
            req.enabled as i64,
            Utc::now().to_rfc3339(),
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn lock_error<E: std::fmt::Display>(e: E) -> AppError {
    AppError {
        code: "DB_LOCK_FAILED".into(),
        message: "Could not access the local database.".into(),
        details: Some(e.to_string()),
        recoverable: true,
    }
}

fn db_error<E: std::fmt::Display>(e: E) -> AppError {
    AppError {
        code: "DB_QUERY_FAILED".into(),
        message: "Could not read firewall rules from the local database.".into(),
        details: Some(e.to_string()),
        recoverable: true,
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use windows::core::BSTR;
    use windows::Win32::Foundation::VARIANT_BOOL;
    use windows::Win32::NetworkManagement::WindowsFirewall::{
        INetFwPolicy2, INetFwRule, NetFwPolicy2, NetFwRule, NET_FW_ACTION_ALLOW,
        NET_FW_ACTION_BLOCK, NET_FW_PROFILE2_ALL, NET_FW_RULE_DIR_IN, NET_FW_RULE_DIR_OUT,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    };

    /// `NET_FW_IP_PROTOCOL_ANY`. Not exposed as a named constant by the
    /// `windows` crate's WindowsFirewall bindings at the time of writing —
    /// double-check against the pinned crate version during the Windows
    /// compile pass (see phase 4 handoff checklist).
    const NET_FW_IP_PROTOCOL_ANY: i32 = 256;

    fn policy() -> Result<INetFwPolicy2, AppError> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            CoCreateInstance(&NetFwPolicy2, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| com_error("Could not access Windows Firewall.", e))
        }
    }

    pub fn add_rule(req: &CreateFirewallRuleRequest) -> Result<(), AppError> {
        unsafe {
            let policy = policy()?;
            let rule: INetFwRule = CoCreateInstance(&NetFwRule, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| com_error("Could not create a firewall rule object.", e))?;

            rule.SetName(&BSTR::from(req.name.as_str()))
                .map_err(|e| com_error("Could not name the firewall rule.", e))?;
            if let Some(desc) = &req.description {
                rule.SetDescription(&BSTR::from(desc.as_str())).ok();
            }
            rule.SetProtocol(match req.protocol {
                FirewallProtocol::Tcp => 6,  // IPPROTO_TCP
                FirewallProtocol::Udp => 17, // IPPROTO_UDP
                FirewallProtocol::Any => NET_FW_IP_PROTOCOL_ANY,
            })
            .map_err(|e| com_error("Could not set the rule protocol.", e))?;

            if let Some(local) = &req.local_ports {
                if !local.is_empty() {
                    rule.SetLocalPorts(&BSTR::from(local.as_str()))
                        .map_err(|e| com_error("Could not set local ports.", e))?;
                }
            }
            if let Some(remote_ports) = &req.remote_ports {
                if !remote_ports.is_empty() {
                    rule.SetRemotePorts(&BSTR::from(remote_ports.as_str()))
                        .map_err(|e| com_error("Could not set remote ports.", e))?;
                }
            }
            if let Some(remote_addr) = &req.remote_addresses {
                if !remote_addr.is_empty() {
                    rule.SetRemoteAddresses(&BSTR::from(remote_addr.as_str()))
                        .map_err(|e| com_error("Could not set remote addresses.", e))?;
                }
            }
            if let Some(app) = &req.application_path {
                if !app.is_empty() {
                    rule.SetApplicationName(&BSTR::from(app.as_str()))
                        .map_err(|e| com_error("Could not scope the rule to an application.", e))?;
                }
            }

            rule.SetDirection(match req.direction {
                PortDirection::Inbound => NET_FW_RULE_DIR_IN,
                PortDirection::Outbound => NET_FW_RULE_DIR_OUT,
            })
            .map_err(|e| com_error("Could not set the rule direction.", e))?;
            rule.SetAction(match req.action {
                FirewallAction::Allow => NET_FW_ACTION_ALLOW,
                FirewallAction::Block => NET_FW_ACTION_BLOCK,
            })
            .map_err(|e| com_error("Could not set the rule action.", e))?;
            rule.SetEnabled(VARIANT_BOOL::from(req.enabled))
                .map_err(|e| com_error("Could not set the rule's enabled state.", e))?;
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

    pub fn set_enabled(name: &str, enabled: bool) -> Result<(), AppError> {
        unsafe {
            let policy = policy()?;
            let rules = policy
                .Rules()
                .map_err(|e| com_error("Could not read firewall rules.", e))?;
            let rule = rules
                .Item(&BSTR::from(name))
                .map_err(|e| com_error("Could not find that firewall rule.", e))?;
            rule.SetEnabled(VARIANT_BOOL::from(enabled))
                .map_err(|e| com_error("Could not change the rule's enabled state.", e))?;
            Ok(())
        }
    }

    pub fn remove_rule(name: &str) -> Result<(), AppError> {
        unsafe {
            let policy = policy()?;
            let rules = policy
                .Rules()
                .map_err(|e| com_error("Could not read firewall rules.", e))?;
            rules
                .Remove(&BSTR::from(name))
                .map_err(|e| com_error("Could not remove that firewall rule.", e))?;
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
}
