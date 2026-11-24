use rusqlite::Connection;

use crate::domain::models::{DashboardMetrics, SeverityCounts, TestProgress};
use crate::error::AppResult;

pub fn dashboard(conn: &Connection) -> AppResult<DashboardMetrics> {
    let total_engagements: i64 =
        conn.query_row("SELECT COUNT(*) FROM engagements", [], |row| row.get(0))?;
    let active_engagements: i64 = conn.query_row(
        "SELECT COUNT(*) FROM engagements WHERE status = 'active'",
        [],
        |row| row.get(0),
    )?;
    let open_findings: i64 = conn.query_row(
        "SELECT COUNT(*) FROM findings WHERE status IN ('open', 'in_review', 'retest_pending')",
        [],
        |row| row.get(0),
    )?;

    let mut findings_by_severity = SeverityCounts::default();
    let mut stmt = conn.prepare(
        "SELECT severity, COUNT(*) FROM findings
         WHERE status IN ('open', 'in_review', 'retest_pending')
         GROUP BY severity",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (severity, count) = row?;
        match severity.as_str() {
            "informational" => findings_by_severity.informational = count,
            "low" => findings_by_severity.low = count,
            "medium" => findings_by_severity.medium = count,
            "high" => findings_by_severity.high = count,
            "critical" => findings_by_severity.critical = count,
            _ => {}
        }
    }

    let mut test_progress = TestProgress::default();
    let mut stmt = conn.prepare("SELECT status, COUNT(*) FROM test_cases GROUP BY status")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (status, count) = row?;
        test_progress.total += count;
        match status.as_str() {
            "not_started" => test_progress.not_started = count,
            "in_progress" => test_progress.in_progress = count,
            "done" => test_progress.done = count,
            "blocked" => test_progress.blocked = count,
            "not_applicable" => test_progress.not_applicable = count,
            _ => {}
        }
    }

    Ok(DashboardMetrics {
        total_engagements,
        active_engagements,
        open_findings,
        findings_by_severity,
        test_progress,
    })
}
