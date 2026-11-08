use rusqlite::{Connection, OptionalExtension};

use crate::domain::enums::{FindingStatus, RetestStatus, Severity};
use crate::domain::models::{Finding, FindingInput};
use crate::error::{AppError, AppResult};

type FindingRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    u8,
    u8,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);

const SELECT: &str = "SELECT id, engagement_id, code, title, description, impact, remediation,
    severity, probability, impact_score, owasp, cwe, status, retest_status, owner, created_at,
    updated_at FROM findings";

pub fn list(conn: &Connection, engagement_id: &str) -> AppResult<Vec<Finding>> {
    let mut stmt = conn.prepare(&format!(
        "{SELECT} WHERE engagement_id = ?1 ORDER BY created_at DESC"
    ))?;
    let rows = stmt
        .query_map([engagement_id], map_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(|row| hydrate(conn, row))
        .collect::<AppResult<Vec<_>>>()
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Finding> {
    let row = conn
        .query_row(&format!("{SELECT} WHERE id = ?1"), [id], map_row)
        .optional()?
        .ok_or(AppError::NotFound)?;
    hydrate(conn, row)
}

pub fn create(
    conn: &Connection,
    input: &FindingInput,
    severity: Severity,
    code: &str,
    now: &str,
) -> AppResult<Finding> {
    let id = uuid::Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO findings (
            id, engagement_id, code, title, description, impact, remediation, severity,
            probability, impact_score, owasp, cwe, status, retest_status, owner, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)",
        rusqlite::params![
            id,
            input.engagement_id,
            code,
            input.title.trim(),
            input.description,
            input.impact,
            input.remediation,
            severity.as_str(),
            input.probability,
            input.impact_score,
            input.owasp.trim(),
            input.cwe.trim(),
            input.status.as_str(),
            input.retest_status.as_str(),
            input.owner.trim(),
            now,
        ],
    )?;
    replace_links(conn, &id, &input.test_case_ids)?;
    get(conn, &id)
}

pub fn update(
    conn: &Connection,
    id: &str,
    input: &FindingInput,
    severity: Severity,
    now: &str,
) -> AppResult<Finding> {
    let changed = conn.execute(
        "UPDATE findings SET
            code = ?1, title = ?2, description = ?3, impact = ?4, remediation = ?5,
            severity = ?6, probability = ?7, impact_score = ?8, owasp = ?9, cwe = ?10,
            status = ?11, retest_status = ?12, owner = ?13, updated_at = ?14
         WHERE id = ?15 AND engagement_id = ?16",
        rusqlite::params![
            input.code.trim(),
            input.title.trim(),
            input.description,
            input.impact,
            input.remediation,
            severity.as_str(),
            input.probability,
            input.impact_score,
            input.owasp.trim(),
            input.cwe.trim(),
            input.status.as_str(),
            input.retest_status.as_str(),
            input.owner.trim(),
            now,
            id,
            input.engagement_id,
        ],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    replace_links(conn, id, &input.test_case_ids)?;
    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let changed = conn.execute("DELETE FROM findings WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub fn next_code(conn: &Connection, engagement_id: &str) -> AppResult<String> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM findings WHERE engagement_id = ?1",
        [engagement_id],
        |row| row.get(0),
    )?;
    Ok(format!("F-{:03}", count + 1))
}

fn replace_links(conn: &Connection, finding_id: &str, test_case_ids: &[String]) -> AppResult<()> {
    conn.execute(
        "DELETE FROM finding_test_cases WHERE finding_id = ?1",
        [finding_id],
    )?;
    for test_case_id in test_case_ids {
        conn.execute(
            "INSERT OR IGNORE INTO finding_test_cases (finding_id, test_case_id) VALUES (?1, ?2)",
            rusqlite::params![finding_id, test_case_id],
        )?;
    }
    Ok(())
}

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FindingRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
        row.get(12)?,
        row.get(13)?,
        row.get(14)?,
        row.get(15)?,
        row.get(16)?,
    ))
}

fn hydrate(conn: &Connection, row: FindingRow) -> AppResult<Finding> {
    let (
        id,
        engagement_id,
        code,
        title,
        description,
        impact,
        remediation,
        severity,
        probability,
        impact_score,
        owasp,
        cwe,
        status,
        retest_status,
        owner,
        created_at,
        updated_at,
    ) = row;
    let mut stmt =
        conn.prepare("SELECT test_case_id FROM finding_test_cases WHERE finding_id = ?1")?;
    let test_case_ids = stmt
        .query_map([&id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(Finding {
        id,
        engagement_id,
        code,
        title,
        description,
        impact,
        remediation,
        severity: Severity::from_db(&severity)?,
        probability,
        impact_score,
        owasp,
        cwe,
        status: FindingStatus::from_db(&status)?,
        retest_status: RetestStatus::from_db(&retest_status)?,
        owner,
        created_at,
        updated_at,
        test_case_ids,
    })
}
