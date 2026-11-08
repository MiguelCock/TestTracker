use rusqlite::{Connection, OptionalExtension};

use crate::domain::enums::TestCaseStatus;
use crate::domain::models::{TestCase, TestCaseInput, TestCaseTemplate};
use crate::error::{AppError, AppResult};

pub fn list(conn: &Connection, engagement_id: &str) -> AppResult<Vec<TestCase>> {
    let mut stmt = conn.prepare(
        "SELECT id, engagement_id, template_id, title, approach, category, status, result,
                notes, assignee, sort_order, created_at, updated_at
         FROM test_cases WHERE engagement_id = ?1
         ORDER BY sort_order ASC, created_at ASC",
    )?;
    let rows = stmt.query_map([engagement_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, i64>(10)?,
            row.get::<_, String>(11)?,
            row.get::<_, String>(12)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (
            id,
            engagement_id,
            template_id,
            title,
            approach,
            category,
            status,
            result,
            notes,
            assignee,
            sort_order,
            created_at,
            updated_at,
        ) = row?;
        out.push(TestCase {
            id,
            engagement_id,
            template_id,
            title,
            approach,
            category,
            status: TestCaseStatus::from_db(&status)?,
            result,
            notes,
            assignee,
            sort_order,
            created_at,
            updated_at,
        });
    }
    Ok(out)
}

pub fn templates(conn: &Connection, domain: Option<&str>) -> AppResult<Vec<TestCaseTemplate>> {
    let (sql, params): (&str, Vec<&str>) = match domain {
        Some(domain) if !domain.trim().is_empty() => (
            "SELECT id, domain, category, title, approach, expected_result, references_text
             FROM test_case_templates WHERE domain = ?1 ORDER BY domain, category, title",
            vec![domain],
        ),
        _ => (
            "SELECT id, domain, category, title, approach, expected_result, references_text
             FROM test_case_templates ORDER BY domain, category, title",
            vec![],
        ),
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
        Ok(TestCaseTemplate {
            id: row.get(0)?,
            domain: row.get(1)?,
            category: row.get(2)?,
            title: row.get(3)?,
            approach: row.get(4)?,
            expected_result: row.get(5)?,
            references: row.get(6)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn create(conn: &Connection, input: &TestCaseInput, now: &str) -> AppResult<TestCase> {
    ensure_engagement(conn, &input.engagement_id)?;
    let id = uuid::Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO test_cases (
            id, engagement_id, template_id, title, approach, category, status, result,
            notes, assignee, sort_order, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
        rusqlite::params![
            id,
            input.engagement_id,
            input.template_id,
            input.title.trim(),
            input.approach,
            input.category.trim(),
            input.status.as_str(),
            input.result,
            input.notes,
            input.assignee.trim(),
            input.sort_order,
            now,
        ],
    )?;
    get(conn, &id)
}

pub fn update(
    conn: &Connection,
    id: &str,
    input: &TestCaseInput,
    now: &str,
) -> AppResult<TestCase> {
    let changed = conn.execute(
        "UPDATE test_cases SET
            template_id = ?1, title = ?2, approach = ?3, category = ?4, status = ?5,
            result = ?6, notes = ?7, assignee = ?8, sort_order = ?9, updated_at = ?10
         WHERE id = ?11 AND engagement_id = ?12",
        rusqlite::params![
            input.template_id,
            input.title.trim(),
            input.approach,
            input.category.trim(),
            input.status.as_str(),
            input.result,
            input.notes,
            input.assignee.trim(),
            input.sort_order,
            now,
            id,
            input.engagement_id,
        ],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let changed = conn.execute("DELETE FROM test_cases WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> AppResult<TestCase> {
    let row = conn
        .query_row(
            "SELECT id, engagement_id, template_id, title, approach, category, status, result,
                    notes, assignee, sort_order, created_at, updated_at
             FROM test_cases WHERE id = ?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, String>(12)?,
                ))
            },
        )
        .optional()?;
    match row {
        Some((
            id,
            engagement_id,
            template_id,
            title,
            approach,
            category,
            status,
            result,
            notes,
            assignee,
            sort_order,
            created_at,
            updated_at,
        )) => Ok(TestCase {
            id,
            engagement_id,
            template_id,
            title,
            approach,
            category,
            status: TestCaseStatus::from_db(&status)?,
            result,
            notes,
            assignee,
            sort_order,
            created_at,
            updated_at,
        }),
        None => Err(AppError::NotFound),
    }
}

pub fn ensure_engagement(conn: &Connection, engagement_id: &str) -> AppResult<()> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM engagements WHERE id = ?1",
            [engagement_id],
            |row| row.get(0),
        )
        .optional()?;
    if exists.is_none() {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub fn validate_links(conn: &Connection, engagement_id: &str, ids: &[String]) -> AppResult<()> {
    for id in ids {
        let exists: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM test_cases WHERE id = ?1 AND engagement_id = ?2",
                rusqlite::params![id, engagement_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(AppError::Validation(
                "A linked test case does not belong to this engagement.".into(),
            ));
        }
    }
    Ok(())
}

pub fn get_template(conn: &Connection, id: &str) -> AppResult<TestCaseTemplate> {
    let row = conn
        .query_row(
            "SELECT id, domain, category, title, approach, expected_result, references_text
             FROM test_case_templates WHERE id = ?1",
            [id],
            |row| {
                Ok(TestCaseTemplate {
                    id: row.get(0)?,
                    domain: row.get(1)?,
                    category: row.get(2)?,
                    title: row.get(3)?,
                    approach: row.get(4)?,
                    expected_result: row.get(5)?,
                    references: row.get(6)?,
                })
            },
        )
        .optional()?;
    row.ok_or(AppError::NotFound)
}

pub fn template_exists(conn: &Connection, id: &str) -> AppResult<bool> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM test_case_templates WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(exists.is_some())
}
