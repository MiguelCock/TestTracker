use rusqlite::{Connection, OptionalExtension};

use crate::domain::models::TestTemplateBundle;
use crate::error::{AppError, AppResult};

pub fn list_bundles(conn: &Connection) -> AppResult<Vec<TestTemplateBundle>> {
    let mut stmt = conn.prepare(
        "SELECT b.id, b.name, b.description, b.version, b.source, b.origin_id, b.updated_at,
                (SELECT COUNT(*) FROM test_template_bundle_items i WHERE i.bundle_id = b.id)
         FROM test_template_bundles b
         ORDER BY b.source ASC, b.name ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(TestTemplateBundle {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            version: row.get(3)?,
            source: row.get(4)?,
            origin_id: row.get(5)?,
            updated_at: row.get(6)?,
            template_count: row.get(7)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get_bundle(conn: &Connection, id: &str) -> AppResult<TestTemplateBundle> {
    let mut stmt = conn.prepare(
        "SELECT b.id, b.name, b.description, b.version, b.source, b.origin_id, b.updated_at,
                (SELECT COUNT(*) FROM test_template_bundle_items i WHERE i.bundle_id = b.id)
         FROM test_template_bundles b WHERE b.id = ?1",
    )?;
    stmt.query_row([id], |row| {
        Ok(TestTemplateBundle {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            version: row.get(3)?,
            source: row.get(4)?,
            origin_id: row.get(5)?,
            updated_at: row.get(6)?,
            template_count: row.get(7)?,
        })
    })
    .optional()?
    .ok_or(AppError::NotFound)
}

pub fn bundle_template_ids(conn: &Connection, bundle_id: &str) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT template_id FROM test_template_bundle_items
         WHERE bundle_id = ?1 ORDER BY sort_order ASC, template_id ASC",
    )?;
    let rows = stmt.query_map([bundle_id], |row| row.get(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn find_imported_by_origin(conn: &Connection, origin_id: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT id FROM test_template_bundles
             WHERE source = 'imported' AND origin_id = ?1",
            [origin_id],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn delete_imported_bundle(conn: &Connection, id: &str) -> AppResult<()> {
    let changed = conn.execute(
        "DELETE FROM test_template_bundles WHERE id = ?1 AND source = 'imported'",
        [id],
    )?;
    if changed == 0 {
        return Err(AppError::Validation(
            "Only imported template bundles can be deleted.".into(),
        ));
    }
    delete_orphan_imported_templates(conn)?;
    Ok(())
}

pub fn delete_orphan_imported_templates(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "DELETE FROM test_case_templates
         WHERE source = 'imported'
           AND id NOT IN (SELECT template_id FROM test_template_bundle_items)",
        [],
    )?;
    Ok(())
}

pub fn bundle_export_items(conn: &Connection, bundle_id: &str) -> AppResult<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT template_id, origin_template_id FROM test_template_bundle_items
         WHERE bundle_id = ?1 ORDER BY sort_order ASC, template_id ASC",
    )?;
    let rows = stmt.query_map([bundle_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}
