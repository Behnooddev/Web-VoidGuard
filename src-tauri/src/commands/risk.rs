use crate::db::Db;
use crate::models::{Confidence, RiskFinding, Severity};
use chrono::{Duration, Utc};
use rusqlite::params;
use uuid::Uuid;

/// Runs the correlation rules against recent data and persists any
/// new findings. Intentionally simple for this first pass — each
/// rule is a standalone function so later phases can add more
/// without touching existing ones. Every finding carries its
/// evidence and a confidence level; nothing is a bare score. See
/// SECURITY.md's detection philosophy.
#[tauri::command]
pub fn run_risk_analysis(db: tauri::State<Db>) -> Result<Vec<RiskFinding>, String> {
    let mut findings = Vec::new();
    findings.extend(rule_suspicious_startup_with_recent_network(&db)?);
    findings.extend(rule_multiple_new_startup_entries(&db)?);

    let conn = db.0.lock().map_err(|e| e.to_string())?;
    for f in &findings {
        conn.execute(
            "INSERT OR IGNORE INTO risk_findings (id, timestamp, title, description, severity, confidence, evidence, remediation, related_event_ids)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                f.id,
                f.timestamp.to_rfc3339(),
                f.title,
                f.description,
                serde_json::to_string(&f.severity).unwrap(),
                serde_json::to_string(&f.confidence).unwrap(),
                serde_json::to_string(&f.evidence).unwrap(),
                f.remediation,
                serde_json::to_string(&f.related_event_ids).unwrap(),
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(findings)
}

#[tauri::command]
pub fn get_recent_risk_findings(db: tauri::State<Db>, limit: u32) -> Result<Vec<RiskFinding>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, timestamp, title, description, severity, confidence, evidence, remediation, related_event_ids
             FROM risk_findings ORDER BY timestamp DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![limit], |row| {
            let severity: String = row.get(4)?;
            let confidence: String = row.get(5)?;
            let evidence: String = row.get(6)?;
            let related: String = row.get(8)?;
            Ok(RiskFinding {
                id: row.get(0)?,
                timestamp: row.get::<_, String>(1)?.parse().unwrap_or_else(|_| Utc::now()),
                title: row.get(2)?,
                description: row.get(3)?,
                severity: serde_json::from_str(&severity).unwrap(),
                confidence: serde_json::from_str(&confidence).unwrap(),
                evidence: serde_json::from_str(&evidence).unwrap(),
                remediation: row.get(7)?,
                related_event_ids: serde_json::from_str(&related).unwrap(),
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Rule: a startup entry classified Suspicious/Unknown combined with
/// a port having opened in the same recent window is a stronger
/// signal than either alone — the classic "persistence + network
/// callback" pattern, without asserting either signal alone is
/// malicious.
fn rule_suspicious_startup_with_recent_network(db: &Db) -> Result<Vec<RiskFinding>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let cutoff = (Utc::now() - Duration::hours(24)).to_rfc3339();

    let mut stmt = conn
        .prepare(
            "SELECT name, command, classification FROM startup_entries
             WHERE classification IN ('SUSPICIOUS', 'UNKNOWN') AND last_seen > ?1",
        )
        .map_err(|e| e.to_string())?;
    let suspicious_startups: Vec<(String, String, String)> = stmt
        .query_map(params![cutoff], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;

    let recent_port_opens: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE category = '\"PORT_OPENED\"' AND timestamp > ?1",
            params![cutoff],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let mut findings = Vec::new();
    if recent_port_opens > 0 {
        for (name, command, classification) in suspicious_startups {
            let severity = if classification == "SUSPICIOUS" {
                Severity::High
            } else {
                Severity::Medium
            };
            findings.push(RiskFinding {
                id: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                title: format!("Startup entry '{name}' combined with new network activity"),
                description: format!(
                    "'{name}' ({command}) was flagged {classification} and at least one new listening port was observed in the same 24-hour window."
                ),
                severity,
                confidence: Confidence::Medium,
                evidence: vec![
                    format!("Startup entry classification: {classification}"),
                    format!("{recent_port_opens} new listening port(s) observed in the last 24 hours"),
                ],
                remediation: Some(
                    "Review the startup entry's target executable and the newly opened port(s) in the Network page. Remove the entry if you don't recognize it.".into(),
                ),
                related_event_ids: vec![],
            });
        }
    }
    Ok(findings)
}

/// Rule: several new (never-seen-before) startup entries appearing in
/// a short window is itself a signal, independent of any individual
/// entry's classification — could indicate a bulk install or an
/// automated persistence mechanism.
fn rule_multiple_new_startup_entries(db: &Db) -> Result<Vec<RiskFinding>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let cutoff = (Utc::now() - Duration::hours(1)).to_rfc3339();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM startup_entries WHERE first_seen > ?1",
            params![cutoff],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if count >= 3 {
        Ok(vec![RiskFinding {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            title: "Multiple new startup entries in a short window".into(),
            description: format!(
                "{count} new startup entries were first observed within the last hour."
            ),
            severity: Severity::Medium,
            confidence: Confidence::Low,
            evidence: vec![format!("{count} new entries in under an hour")],
            remediation: Some(
                "If you didn't just install software, review the Startup page for anything unrecognized.".into(),
            ),
            related_event_ids: vec![],
        }])
    } else {
        Ok(vec![])
    }
}
