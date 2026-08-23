use crate::commands::audit::record_audit;
use crate::commands::events::record_event;
use crate::db::Db;
use crate::models::{
    AppError, AuditResult, ChangeStartupTypeRequest, EventCategory, ServiceAction,
    ServiceActionRequest, ServiceInfo, ServiceStatus, Severity, StartupType,
};

/// Services Windows itself depends on for basic stability. Not an
/// exhaustive list — a starting, conservative set that forces the
/// UI's strongest confirmation dialog. Anything not on this list can
/// still be stopped, just with the normal (not extra-strength)
/// confirmation.
const PROTECTED_SERVICES: &[&str] = &[
    "TrustedInstaller",
    "WinDefend",
    "wscsvc",
    "SamSs",
    "gpsvc",
    "RpcSs",
    "DcomLaunch",
    "LSM",
    "Winmgmt",
    "EventLog",
    "Dnscache",
];

#[tauri::command]
pub fn list_services() -> Result<Vec<ServiceInfo>, AppError> {
    #[cfg(windows)]
    {
        windows_impl::enumerate_services()
    }
    #[cfg(not(windows))]
    {
        Err(AppError::not_supported("Service enumeration"))
    }
}

#[tauri::command]
pub fn control_service(db: tauri::State<Db>, req: ServiceActionRequest) -> Result<(), AppError> {
    #[cfg(windows)]
    let result = windows_impl::control_service(&req);
    #[cfg(not(windows))]
    let result: Result<(), AppError> = Err(AppError::not_supported("Service control"));

    let action_name = format!("{:?}_SERVICE", req.action).to_uppercase();
    let _ = record_audit(
        &db,
        &action_name,
        &req.service_name,
        None,
        None,
        match &result {
            Ok(_) => AuditResult::Success,
            Err(_) => AuditResult::Failure,
        },
        "services",
    );
    if result.is_ok() {
        let _ = record_event(
            &db,
            EventCategory::ServiceChanged,
            Severity::Medium,
            "services",
            &format!("Service '{}' was {:?}ed by the user", req.service_name, req.action),
            Some(req.service_name.clone()),
        );
    }
    result
}

#[tauri::command]
pub fn change_service_startup_type(
    db: tauri::State<Db>,
    req: ChangeStartupTypeRequest,
) -> Result<(), AppError> {
    #[cfg(windows)]
    let result = windows_impl::change_startup_type(&req);
    #[cfg(not(windows))]
    let result: Result<(), AppError> = Err(AppError::not_supported("Service startup type change"));

    let _ = record_audit(
        &db,
        "CHANGE_SERVICE_STARTUP_TYPE",
        &req.service_name,
        None,
        Some(format!("{:?}", req.startup_type)),
        match &result {
            Ok(_) => AuditResult::Success,
            Err(_) => AuditResult::Failure,
        },
        "services",
    );
    if result.is_ok() {
        let _ = record_event(
            &db,
            EventCategory::ServiceChanged,
            Severity::Medium,
            "services",
            &format!(
                "Startup type for '{}' changed to {:?}",
                req.service_name, req.startup_type
            ),
            Some(req.service_name.clone()),
        );
    }
    result
}

fn is_protected(name: &str) -> bool {
    PROTECTED_SERVICES
        .iter()
        .any(|p| p.eq_ignore_ascii_case(name))
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use windows::Win32::Security::SC_HANDLE;
    use windows::Win32::System::Services::{
        ChangeServiceConfigW, CloseServiceHandle, ControlService, EnumServicesStatusExW,
        OpenSCManagerW, OpenServiceW, QueryServiceConfigW, StartServiceW,
        ENUM_SERVICE_STATUS_PROCESSW, ENUM_SERVICE_TYPE, QUERY_SERVICE_CONFIGW,
        SC_ENUM_PROCESS_INFO, SC_MANAGER_ENUMERATE_SERVICE, SERVICE_AUTO_START,
        SERVICE_CONTROL_STOP, SERVICE_DEMAND_START, SERVICE_DISABLED,
        SERVICE_ERROR, SERVICE_NO_CHANGE, SERVICE_QUERY_CONFIG, SERVICE_QUERY_STATUS,
        SERVICE_RUNNING, SERVICE_START, SERVICE_START_PENDING, SERVICE_STATE_ALL, SERVICE_STATUS,
        SERVICE_STATUS_CURRENT_STATE, SERVICE_STOP, SERVICE_STOPPED, SERVICE_STOP_PENDING,
        SERVICE_WIN32,
    };

    /// `ChangeServiceConfigW`'s "leave this unchanged" sentinel
    /// (`SERVICE_NO_CHANGE`, `0xFFFFFFFF`) is exposed by the `windows`
    /// crate as a bare `u32`, but the parameters that take it
    /// (`dwServiceType`/`dwErrorControl`) are typed `ENUM_SERVICE_TYPE`/
    /// `SERVICE_ERROR` — these wrap it in the right type for each.
    const NO_CHANGE_TYPE: ENUM_SERVICE_TYPE = ENUM_SERVICE_TYPE(SERVICE_NO_CHANGE);
    const NO_CHANGE_ERROR: SERVICE_ERROR = SERVICE_ERROR(SERVICE_NO_CHANGE);

    pub fn enumerate_services() -> Result<Vec<ServiceInfo>, AppError> {
        unsafe {
            let scm = OpenSCManagerW(None, None, SC_MANAGER_ENUMERATE_SERVICE)
                .map_err(|e| scm_error("Could not open the Service Control Manager.", e))?;

            let mut bytes_needed: u32 = 0;
            let mut services_returned: u32 = 0;
            let mut resume_handle: u32 = 0;

            // First call to learn the required buffer size.
            let _ = EnumServicesStatusExW(
                scm,
                SC_ENUM_PROCESS_INFO,
                SERVICE_WIN32,
                SERVICE_STATE_ALL,
                None,
                &mut bytes_needed,
                &mut services_returned,
                Some(&mut resume_handle),
                None,
            );

            let mut buffer = vec![0u8; bytes_needed as usize];
            let ok = EnumServicesStatusExW(
                scm,
                SC_ENUM_PROCESS_INFO,
                SERVICE_WIN32,
                SERVICE_STATE_ALL,
                Some(&mut buffer),
                &mut bytes_needed,
                &mut services_returned,
                Some(&mut resume_handle),
                None,
            );
            if ok.is_err() {
                let _ = CloseServiceHandle(scm);
                return Err(AppError {
                    code: "SERVICE_ENUM_FAILED".into(),
                    message: "Failed to enumerate Windows services.".into(),
                    details: None,
                    recoverable: true,
                });
            }

            let entries = std::slice::from_raw_parts(
                buffer.as_ptr() as *const ENUM_SERVICE_STATUS_PROCESSW,
                services_returned as usize,
            );

            let mut result = Vec::with_capacity(entries.len());
            for entry in entries {
                let name = pwstr_to_string(entry.lpServiceName.0);
                let display_name = pwstr_to_string(entry.lpDisplayName.0);
                let status = map_status(entry.ServiceStatusProcess.dwCurrentState);
                let (startup_type, executable) = query_config(scm, &name);

                result.push(ServiceInfo {
                    protected: is_protected(&name),
                    name,
                    display_name,
                    status,
                    startup_type,
                    executable,
                    description: None, // SERVICE_CONFIG_DESCRIPTION lookup omitted for brevity of this pass
                });
            }

            let _ = CloseServiceHandle(scm);
            Ok(result)
        }
    }

    unsafe fn query_config(scm: SC_HANDLE, name: &str) -> (StartupType, Option<String>) {
        let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let Ok(handle) = OpenServiceW(
            scm,
            windows::core::PCWSTR(wide.as_ptr()),
            SERVICE_QUERY_CONFIG,
        ) else {
            return (StartupType::Unknown, None);
        };

        let mut needed = 0u32;
        let _ = QueryServiceConfigW(handle, None, 0, &mut needed);
        let mut buf = vec![0u8; needed as usize];
        let ok = QueryServiceConfigW(
            handle,
            Some(buf.as_mut_ptr() as *mut QUERY_SERVICE_CONFIGW),
            needed,
            &mut needed,
        );

        let result = if ok.is_ok() {
            let cfg = &*(buf.as_ptr() as *const QUERY_SERVICE_CONFIGW);
            let startup = match cfg.dwStartType {
                t if t == SERVICE_AUTO_START => StartupType::Automatic,
                t if t == SERVICE_DEMAND_START => StartupType::Manual,
                t if t == SERVICE_DISABLED => StartupType::Disabled,
                _ => StartupType::Unknown,
            };
            let exe = if !cfg.lpBinaryPathName.is_null() {
                Some(pwstr_to_string(cfg.lpBinaryPathName.0))
            } else {
                None
            };
            (startup, exe)
        } else {
            (StartupType::Unknown, None)
        };

        let _ = CloseServiceHandle(handle);
        result
    }

    fn map_status(state: SERVICE_STATUS_CURRENT_STATE) -> ServiceStatus {
        match state {
            s if s == SERVICE_RUNNING => ServiceStatus::Running,
            s if s == SERVICE_STOPPED => ServiceStatus::Stopped,
            s if s == SERVICE_START_PENDING => ServiceStatus::StartPending,
            s if s == SERVICE_STOP_PENDING => ServiceStatus::StopPending,
            _ => ServiceStatus::Unknown,
        }
    }

    pub fn control_service(req: &ServiceActionRequest) -> Result<(), AppError> {
        unsafe {
            let scm = OpenSCManagerW(None, None, SC_MANAGER_ENUMERATE_SERVICE)
                .map_err(|e| scm_error("Could not open the Service Control Manager.", e))?;

            let wide: Vec<u16> = req
                .service_name
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let access = SERVICE_START | SERVICE_STOP | SERVICE_QUERY_STATUS;
            let handle = OpenServiceW(scm, windows::core::PCWSTR(wide.as_ptr()), access)
                .map_err(|e| scm_error("Could not open the service.", e));
            let _ = CloseServiceHandle(scm);
            let handle = handle?;

            let result = match req.action {
                ServiceAction::Start => StartServiceW(handle, None)
                    .map_err(|e| scm_error("Windows refused to start the service.", e)),
                ServiceAction::Stop => {
                    let mut status = SERVICE_STATUS::default();
                    ControlService(handle, SERVICE_CONTROL_STOP, &mut status)
                        .map(|_| ())
                        .map_err(|e| scm_error("Windows refused to stop the service.", e))
                }
                ServiceAction::Restart => {
                    let mut status = SERVICE_STATUS::default();
                    let stop = ControlService(handle, SERVICE_CONTROL_STOP, &mut status);
                    // Give the SCM a moment; a production implementation
                    // should poll QueryServiceStatusEx instead of a
                    // fixed sleep — flagged for the debugging pass.
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                    let start = StartServiceW(handle, None);
                    match (stop, start) {
                        (_, Ok(_)) => Ok(()),
                        (Ok(_), Err(e)) => Err(scm_error(
                            "The service stopped, but Windows refused to start it again.",
                            e,
                        )),
                        (Err(e), Err(_)) => Err(scm_error(
                            "Windows refused to stop the service, so the restart did not proceed.",
                            e,
                        )),
                    }
                }
            };

            let _ = CloseServiceHandle(handle);
            result
        }
    }

    pub fn change_startup_type(req: &ChangeStartupTypeRequest) -> Result<(), AppError> {
        unsafe {
            let scm = OpenSCManagerW(None, None, SC_MANAGER_ENUMERATE_SERVICE)
                .map_err(|e| scm_error("Could not open the Service Control Manager.", e))?;

            let wide: Vec<u16> = req
                .service_name
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let handle = OpenServiceW(
                scm,
                windows::core::PCWSTR(wide.as_ptr()),
                windows::Win32::System::Services::SERVICE_CHANGE_CONFIG,
            )
            .map_err(|e| scm_error("Could not open the service.", e));
            let _ = CloseServiceHandle(scm);
            let handle = handle?;

            let start_type = match req.startup_type {
                StartupType::Automatic | StartupType::AutomaticDelayed => SERVICE_AUTO_START,
                StartupType::Manual => SERVICE_DEMAND_START,
                StartupType::Disabled => SERVICE_DISABLED,
                StartupType::Unknown => {
                    let _ = CloseServiceHandle(handle);
                    return Err(AppError {
                        code: "INVALID_STARTUP_TYPE".into(),
                        message: "Cannot set startup type to 'Unknown'.".into(),
                        details: None,
                        recoverable: false,
                    });
                }
            };

            let result = ChangeServiceConfigW(
                handle,
                NO_CHANGE_TYPE,
                start_type,
                NO_CHANGE_ERROR,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .map_err(|e| scm_error("Windows rejected the startup type change.", e));

            let _ = CloseServiceHandle(handle);
            result
        }
    }

    unsafe fn pwstr_to_string(ptr: *mut u16) -> String {
        if ptr.is_null() {
            return String::new();
        }
        let mut len = 0usize;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        String::from_utf16_lossy(slice)
    }

    fn scm_error(message: &str, e: windows::core::Error) -> AppError {
        AppError {
            code: "SCM_ERROR".into(),
            message: message.into(),
            details: Some(e.message().to_string()),
            recoverable: true,
        }
    }
}
