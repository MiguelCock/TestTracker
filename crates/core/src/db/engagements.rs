use rusqlite::{Connection, OptionalExtension};

use crate::domain::enums::EngagementStatus;
use crate::domain::models::{
    Assumption, Contact, Engagement, EngagementDetail, EngagementInput, EngagementSummary, Risk,
    ScopeAsset,
};
use crate::domain::severity::severity_for;
use crate::error::{AppError, AppResult};

pub fn list(conn: &Connection) -> AppResult<Vec<EngagementSummary>> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.name, e.client_code, e.status, e.start_date, e.end_date, e.updated_at,
                (SELECT COUNT(*) FROM test_cases tc WHERE tc.engagement_id = e.id),
                (SELECT COUNT(*) FROM findings f WHERE f.engagement_id = e.id),
                (SELECT COUNT(*) FROM findings f WHERE f.engagement_id = e.id
                    AND f.status IN ('open', 'in_review', 'retest_pending'))
         FROM engagements e
         ORDER BY e.updated_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, i64>(7)?,
            row.get::<_, i64>(8)?,
            row.get::<_, i64>(9)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (
            id,
            name,
            client_code,
            status,
            start_date,
            end_date,
            updated_at,
            test_case_count,
            finding_count,
            open_finding_count,
        ) = row?;
        out.push(EngagementSummary {
            id,
            name,
            client_code,
            status: EngagementStatus::from_db(&status)?,
            start_date,
            end_date,
            updated_at,
            test_case_count,
            finding_count,
            open_finding_count,
        });
    }
    Ok(out)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Engagement> {
    let row = conn
        .query_row(
            "SELECT id, name, client_code, status, start_date, end_date, application_summary,
                    attack_surface, created_by, created_at, updated_at, version
             FROM engagements WHERE id = ?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, i64>(11)?,
                ))
            },
        )
        .optional()?;
    match row {
        Some((
            id,
            name,
            client_code,
            status,
            start_date,
            end_date,
            application_summary,
            attack_surface,
            created_by,
            created_at,
            updated_at,
            version,
        )) => Ok(Engagement {
            id,
            name,
            client_code,
            status: EngagementStatus::from_db(&status)?,
            start_date,
            end_date,
            application_summary,
            attack_surface,
            created_by,
            created_at,
            updated_at,
            version,
        }),
        None => Err(AppError::NotFound),
    }
}

pub fn detail(conn: &Connection, id: &str) -> AppResult<EngagementDetail> {
    let engagement = get(conn, id)?;

    let mut stmt = conn.prepare(
        "SELECT id, kind, value, in_scope, notes FROM scope_assets WHERE engagement_id = ?1 ORDER BY rowid",
    )?;
    let scope_assets = stmt
        .query_map([id], |row| {
            Ok(ScopeAsset {
                id: row.get(0)?,
                kind: row.get(1)?,
                value: row.get(2)?,
                in_scope: row.get::<_, i64>(3)? != 0,
                notes: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut stmt = conn.prepare(
        "SELECT id, title, description, likelihood, impact, severity, notes
         FROM risks WHERE engagement_id = ?1 ORDER BY rowid",
    )?;
    let risks = stmt
        .query_map([id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, u8>(3)?,
                row.get::<_, u8>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?
        .map(|row| {
            let (id, title, description, likelihood, impact, severity, notes) = row?;
            Ok(Risk {
                id,
                title,
                description,
                likelihood,
                impact,
                severity: crate::domain::enums::Severity::from_db(&severity)?,
                notes,
            })
        })
        .collect::<AppResult<Vec<_>>>()?;

    let mut stmt =
        conn.prepare("SELECT id, text FROM assumptions WHERE engagement_id = ?1 ORDER BY rowid")?;
    let assumptions = stmt
        .query_map([id], |row| {
            Ok(Assumption {
                id: row.get(0)?,
                text: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut stmt = conn.prepare(
        "SELECT id, name, role, email, phone FROM contacts WHERE engagement_id = ?1 ORDER BY rowid",
    )?;
    let contacts = stmt
        .query_map([id], |row| {
            Ok(Contact {
                id: row.get(0)?,
                name: row.get(1)?,
                role: row.get(2)?,
                email: row.get(3)?,
                phone: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let (test_case_count, finding_count, open_finding_count) = counts(conn, id)?;

    Ok(EngagementDetail {
        engagement,
        scope_assets,
        risks,
        assumptions,
        contacts,
        test_case_count,
        finding_count,
        open_finding_count,
    })
}

pub fn insert(
    conn: &Connection,
    id: &str,
    input: &EngagementInput,
    actor_id: &str,
    now: &str,
) -> AppResult<Engagement> {
    conn.execute(
        "INSERT INTO engagements (
            id, name, client_code, status, start_date, end_date, application_summary,
            attack_surface, created_by, created_at, updated_at, version
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 1)",
        rusqlite::params![
            id,
            input.name.trim(),
            input.client_code.trim(),
            input.status.as_str(),
            input.start_date.trim(),
            input.end_date.trim(),
            input.application_summary,
            input.attack_surface,
            actor_id,
            now,
            now,
        ],
    )?;
    conn.execute(
        "INSERT INTO engagement_members (engagement_id, profile_id, role)
         SELECT ?1, id, role FROM profiles WHERE id = ?2",
        rusqlite::params![id, actor_id],
    )?;
    replace_children(conn, id, input)?;
    get(conn, id)
}

pub fn update(
    conn: &Connection,
    id: &str,
    input: &EngagementInput,
    now: &str,
) -> AppResult<Engagement> {
    let changed = conn.execute(
        "UPDATE engagements SET
            name = ?1, client_code = ?2, status = ?3, start_date = ?4, end_date = ?5,
            application_summary = ?6, attack_surface = ?7, updated_at = ?8, version = version + 1
         WHERE id = ?9",
        rusqlite::params![
            input.name.trim(),
            input.client_code.trim(),
            input.status.as_str(),
            input.start_date.trim(),
            input.end_date.trim(),
            input.application_summary,
            input.attack_surface,
            now,
            id,
        ],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    replace_children(conn, id, input)?;
    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let changed = conn.execute("DELETE FROM engagements WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub fn counts(conn: &Connection, id: &str) -> AppResult<(i64, i64, i64)> {
    let test_case_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM test_cases WHERE engagement_id = ?1",
        [id],
        |row| row.get(0),
    )?;
    let finding_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM findings WHERE engagement_id = ?1",
        [id],
        |row| row.get(0),
    )?;
    let open_finding_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM findings WHERE engagement_id = ?1
         AND status IN ('open', 'in_review', 'retest_pending')",
        [id],
        |row| row.get(0),
    )?;
    Ok((test_case_count, finding_count, open_finding_count))
}

fn replace_children(
    conn: &Connection,
    engagement_id: &str,
    input: &EngagementInput,
) -> AppResult<()> {
    conn.execute(
        "DELETE FROM scope_assets WHERE engagement_id = ?1",
        [engagement_id],
    )?;
    conn.execute(
        "DELETE FROM risks WHERE engagement_id = ?1",
        [engagement_id],
    )?;
    conn.execute(
        "DELETE FROM assumptions WHERE engagement_id = ?1",
        [engagement_id],
    )?;
    conn.execute(
        "DELETE FROM contacts WHERE engagement_id = ?1",
        [engagement_id],
    )?;

    for asset in &input.scope_assets {
        conn.execute(
            "INSERT INTO scope_assets (id, engagement_id, kind, value, in_scope, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                uuid::Uuid::now_v7().to_string(),
                engagement_id,
                asset.kind.trim(),
                asset.value.trim(),
                if asset.in_scope { 1 } else { 0 },
                asset.notes,
            ],
        )?;
    }
    for risk in &input.risks {
        let severity = severity_for(risk.likelihood, risk.impact)?;
        conn.execute(
            "INSERT INTO risks (id, engagement_id, title, description, likelihood, impact, severity, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                uuid::Uuid::now_v7().to_string(),
                engagement_id,
                risk.title.trim(),
                risk.description,
                risk.likelihood,
                risk.impact,
                severity.as_str(),
                risk.notes,
            ],
        )?;
    }
    for text in &input.assumptions {
        if text.trim().is_empty() {
            continue;
        }
        conn.execute(
            "INSERT INTO assumptions (id, engagement_id, text) VALUES (?1, ?2, ?3)",
            rusqlite::params![uuid::Uuid::now_v7().to_string(), engagement_id, text.trim()],
        )?;
    }
    for contact in &input.contacts {
        if contact.name.trim().is_empty() {
            continue;
        }
        conn.execute(
            "INSERT INTO contacts (id, engagement_id, name, role, email, phone)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                uuid::Uuid::now_v7().to_string(),
                engagement_id,
                contact.name.trim(),
                contact.role.trim(),
                contact.email.trim(),
                contact.phone.trim(),
            ],
        )?;
    }
    Ok(())
}
