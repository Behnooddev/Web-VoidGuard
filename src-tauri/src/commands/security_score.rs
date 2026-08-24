use crate::db::Db;
use crate::models::{ScoreReason, SecurityScore, Severity, StartupClassification};
use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

/// Computes the dashboard's security score from currently-available
/// signals. Starts at 100 and subtracts points per concrete reason —
/// every point lost has a label attached, per SECURITY.md's
/// "never a bare score" rule. Persists the result so the dashboard
/// can show the most recent score without recomputing on every
/// render, and so a history is available later (trend chart, etc.).
#[tauri::command]
pub fn compute_security_score(
    app: tauri::AppHandle,
    db: tauri::State<Db>,
) -> Result<SecurityScore, String> {
    let mut score: i32 = 100;
    let mut reasons: Vec<ScoreReason> = Vec::new();

    // Firewall — Windows-native check via the same COM policy object
    // Phase 2/4 already use; a disabled profile is a big signal.
    #[cfg(windows)]
    {
        match windows_impl::firewall_disabled_profiles() {
            Ok(disabled) if !disabled.is_empty() => {
                score -= 20;
                reasons.push(ScoreReason {
                    label: format!("Firewall disabled for: {}", disabled.join(", ")),
                    impact: -20,
                    severity: Severity::High,
                });
            }
            Ok(_) => {}
            Err(_) => {
                // Couldn't read firewall state — don't penalize for a
                // read failure, but don't claim it's fine either.
                reasons.push(ScoreReason {
                    label: "Firewall status could not be checked".into(),
                    impact: 0,
                    severity: Severity::Info,
                });
            }
        }
    }

    // Startup entries — reuse the same classification the Startup
    // page shows, so the score and the page never disagree.
    {
        let state = tauri::Manager::state::<Db>(&app);
        if let Ok(entries) = crate::commands::startup::list_startup_entries(state) {
            let suspicious = entries
                .iter()
                .filter(|e| e.classification == StartupClassification::Suspicious)
                .count();
            let unknown = entries
                .iter()
                .filter(|e| e.classification == StartupClassification::Unknown)
                .count();

            if suspicious > 0 {
                let impact = -(10 * suspicious.min(3)) as i32; // cap at -30
                score += impact;
                reasons.push(ScoreReason {
                    label: format!("{suspicious} suspicious startup entr{}", if suspicious == 1 { "y" } else { "ies" }),
                    impact: impact as i16,
                    severity: Severity::High,
                });
            }
            if unknown > 0 {
                let impact = -(2 * unknown.min(10)) as i32; // cap at -20
                score += impact;
                reasons.push(ScoreReason {
                    label: format!("{unknown} unrecognized (unsigned-status-unknown) startup entr{}", if unknown == 1 { "y" } else { "ies" }),
                    impact: impact as i16,
                    severity: Severity::Medium,
                });
            }
        }
    }

    // Open ports — reuse the risk classification from the Network page.
    if let Ok(ports) = crate::commands::ports::list_listening_ports() {
        let elevated = ports
            .iter()
            .filter(|p| !matches!(p.risk, crate::models::PortRisk::Low))
            .count();
        if elevated > 0 {
            let impact = -(3 * elevated.min(5)) as i32; // cap at -15
            score += impact;
            reasons.push(ScoreReason {
                label: format!("{elevated} listening port(s) outside the well-known/low-risk set"),
                impact: impact as i16,
                severity: Severity::Medium,
            });
        }
    }

    // Recent risk findings from the correlation engine (Phase 3) —
    // don't double-penalize the same evidence twice as heavily, just
    // fold its verdict in as one more input.
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let recent_high: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM risk_findings WHERE severity IN ('\"HIGH\"','\"CRITICAL\"') AND timestamp > datetime('now', '-7 days')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if recent_high > 0 {
            let impact = -(10 * (recent_high as i32).min(2)); // cap at -20
            score += impact;
            reasons.push(ScoreReason {
                label: format!("{recent_high} high/critical risk finding(s) in the last 7 days"),
                impact: impact as i16,
                severity: Severity::High,
            });
        }
    }

    let score = score.clamp(0, 100) as u8;

    if reasons.is_empty() {
        reasons.push(ScoreReason {
            label: "No issues detected across the checks currently implemented".into(),
            impact: 0,
            severity: Severity::Info,
        });
    }

    let result = SecurityScore {
        score,
        reasons,
        calculated_at: Utc::now(),
    };

    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO security_scores (id, calculated_at, score, reasons) VALUES (?1, ?2, ?3, ?4)",
        params![
            Uuid::new_v4().to_string(),
            result.calculated_at.to_rfc3339(),
            result.score as i64,
            serde_json::to_string(&result.reasons).unwrap(),
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
pub fn get_latest_security_score(db: tauri::State<Db>) -> Result<Option<SecurityScore>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let row = conn.query_row(
        "SELECT calculated_at, score, reasons FROM security_scores ORDER BY calculated_at DESC LIMIT 1",
        [],
        |row| {
            let reasons_raw: String = row.get(2)?;
            Ok(SecurityScore {
                calculated_at: row.get::<_, String>(0)?.parse().unwrap_or_else(|_| Utc::now()),
                score: row.get::<_, i64>(1)? as u8,
                reasons: serde_json::from_str(&reasons_raw).unwrap_or_default(),
            })
        },
    );
    match row {
        Ok(score) => Ok(Some(score)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(windows)]
mod windows_impl {
    use windows::Win32::NetworkManagement::WindowsFirewall::{
        INetFwPolicy2, NetFwPolicy2, NET_FW_PROFILE2_DOMAIN, NET_FW_PROFILE2_PRIVATE,
        NET_FW_PROFILE2_PUBLIC,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    };

    pub fn firewall_disabled_profiles() -> windows::core::Result<Vec<String>> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let policy: INetFwPolicy2 =
                CoCreateInstance(&NetFwPolicy2, None, CLSCTX_INPROC_SERVER)?;

            let mut disabled = Vec::new();
            for (profile, label) in [
                (NET_FW_PROFILE2_DOMAIN, "Domain"),
                (NET_FW_PROFILE2_PRIVATE, "Private"),
                (NET_FW_PROFILE2_PUBLIC, "Public"),
            ] {
                let enabled = policy.FirewallEnabled(profile)?;
                if !enabled.as_bool() {
                    disabled.push(label.to_string());
                }
            }
            Ok(disabled)
        }
    }
}
