use rusqlite::{Connection, OptionalExtension};

use crate::domain::models::{EngagementFile, StoredEngagementFile};
use crate::error::{AppError, AppResult};

pub fn list(conn: &Connection, engagement_id: &str) -> AppResult<Vec<EngagementFile>> {
    let mut stmt = conn.prepare(
        "SELECT id, engagement_id, kind, filename, mime_type, size, summary_json, created_by, created_at
         FROM engagement_files WHERE engagement_id = ?1 ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([engagement_id], |row| {
        let summary: String = row.get(6)?;
        Ok(EngagementFile {
            id: row.get(0)?,
            engagement_id: row.get(1)?,
            kind: row.get(2)?,
            filename: row.get(3)?,
            mime_type: row.get(4)?,
            size: row.get(5)?,
            summary: serde_json::from_str(&summary).unwrap_or_else(|_| serde_json::json!({})),
            created_by: row.get(7)?,
            created_at: row.get(8)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<StoredEngagementFile> {
    let row = conn
        .query_row(
            "SELECT id, engagement_id, kind, filename, mime_type, size, summary_json,
                    created_by, created_at, nonce, path, extracted_text, plaintext_hash
             FROM engagement_files WHERE id = ?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, String>(12)?,
                ))
            },
        )
        .optional()?
        .ok_or(AppError::NotFound)?;
    let summary = serde_json::from_str(&row.6).unwrap_or_else(|_| serde_json::json!({}));
    Ok(StoredEngagementFile {
        file: EngagementFile {
            id: row.0,
            engagement_id: row.1,
            kind: row.2,
            filename: row.3,
            mime_type: row.4,
            size: row.5,
            summary,
            created_by: row.7,
            created_at: row.8,
        },
        plaintext_hash: row.12,
        nonce: row.9,
        path: row.10,
        extracted_text: row.11,
    })
}

pub fn insert(conn: &Connection, stored: &StoredEngagementFile) -> AppResult<()> {
    conn.execute(
        "INSERT INTO engagement_files
            (id, engagement_id, kind, filename, mime_type, size, plaintext_hash, nonce, path,
             summary_json, extracted_text, created_by, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        rusqlite::params![
            stored.file.id,
            stored.file.engagement_id,
            stored.file.kind,
            stored.file.filename,
            stored.file.mime_type,
            stored.file.size,
            stored.plaintext_hash,
            stored.nonce,
            stored.path,
            serde_json::to_string(&stored.file.summary)?,
            stored.extracted_text,
            stored.file.created_by,
            stored.file.created_at,
        ],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let changed = conn.execute("DELETE FROM engagement_files WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub fn allow_tool(
    conn: &Connection,
    engagement_id: &str,
    tool: &str,
    actor_id: &str,
    now: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO ai_tool_permissions (engagement_id, tool, created_by, created_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(engagement_id, tool) DO UPDATE SET
            created_by = excluded.created_by,
            created_at = excluded.created_at",
        rusqlite::params![engagement_id, tool, actor_id, now],
    )?;
    Ok(())
}

pub fn revoke_tool(conn: &Connection, engagement_id: &str, tool: &str) -> AppResult<()> {
    conn.execute(
        "DELETE FROM ai_tool_permissions WHERE engagement_id = ?1 AND tool = ?2",
        rusqlite::params![engagement_id, tool],
    )?;
    Ok(())
}

pub fn tool_allowed(conn: &Connection, engagement_id: &str, tool: &str) -> AppResult<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM ai_tool_permissions WHERE engagement_id = ?1 AND tool = ?2",
            rusqlite::params![engagement_id, tool],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

pub fn tool_permissions(
    conn: &Connection,
    engagement_id: &str,
) -> AppResult<Vec<crate::domain::models::ToolPermission>> {
    let mut stmt = conn.prepare(
        "SELECT tool, created_at FROM ai_tool_permissions
         WHERE engagement_id = ?1 ORDER BY tool ASC",
    )?;
    let rows = stmt.query_map([engagement_id], |row| {
        Ok(crate::domain::models::ToolPermission {
            tool: row.get(0)?,
            created_at: row.get(1)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}
