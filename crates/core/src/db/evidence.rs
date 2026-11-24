use rusqlite::{Connection, OptionalExtension};

use crate::domain::models::Evidence;
use crate::error::{AppError, AppResult};

const COLUMNS: &str = "id, engagement_id, finding_id, test_case_id, filename, mime_type, size,
                       blob_id, plaintext_hash, caption, created_by, created_at";

pub fn list(conn: &Connection, engagement_id: &str) -> AppResult<Vec<Evidence>> {
    let sql =
        format!("SELECT {COLUMNS} FROM evidence WHERE engagement_id = ?1 ORDER BY created_at DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map([engagement_id], map_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows.into_iter().map(to_evidence).collect())
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Evidence> {
    let sql = format!("SELECT {COLUMNS} FROM evidence WHERE id = ?1");
    let row = conn
        .query_row(&sql, [id], map_row)
        .optional()?
        .ok_or(AppError::NotFound)?;
    Ok(to_evidence(row))
}

pub fn insert_blob(
    conn: &Connection,
    blob_id: &str,
    engagement_id: &str,
    nonce: &str,
    path: &str,
    created_at: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO evidence_blobs (blob_id, engagement_id, nonce, path, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![blob_id, engagement_id, nonce, path, created_at],
    )?;
    Ok(())
}

pub fn insert(conn: &Connection, evidence: &Evidence) -> AppResult<()> {
    conn.execute(
        "INSERT INTO evidence (
            id, engagement_id, finding_id, test_case_id, filename, mime_type, size,
            blob_id, plaintext_hash, caption, created_by, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        rusqlite::params![
            evidence.id,
            evidence.engagement_id,
            evidence.finding_id,
            evidence.test_case_id,
            evidence.filename,
            evidence.mime_type,
            evidence.size,
            evidence.blob_id,
            evidence.plaintext_hash,
            evidence.caption,
            evidence.created_by,
            evidence.created_at,
        ],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let changed = conn.execute("DELETE FROM evidence WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub fn delete_blob(conn: &Connection, blob_id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM evidence_blobs WHERE blob_id = ?1", [blob_id])?;
    Ok(())
}

pub fn blob(conn: &Connection, blob_id: &str) -> AppResult<(String, String)> {
    conn.query_row(
        "SELECT nonce, path FROM evidence_blobs WHERE blob_id = ?1",
        [blob_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()?
    .ok_or(AppError::NotFound)
}

pub fn link_belongs_to_engagement(
    conn: &Connection,
    table: &str,
    id: &str,
    engagement_id: &str,
) -> AppResult<bool> {
    let sql = format!("SELECT 1 FROM {table} WHERE id = ?1 AND engagement_id = ?2");
    let found: Option<i64> = conn
        .query_row(&sql, rusqlite::params![id, engagement_id], |row| row.get(0))
        .optional()?;
    Ok(found.is_some())
}

type EvidenceRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    String,
    i64,
    String,
    String,
    String,
    String,
    String,
);

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<EvidenceRow> {
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
    ))
}

fn to_evidence(row: EvidenceRow) -> Evidence {
    let (
        id,
        engagement_id,
        finding_id,
        test_case_id,
        filename,
        mime_type,
        size,
        blob_id,
        plaintext_hash,
        caption,
        created_by,
        created_at,
    ) = row;
    Evidence {
        id,
        engagement_id,
        finding_id,
        test_case_id,
        filename,
        mime_type,
        size,
        blob_id,
        plaintext_hash,
        caption,
        created_by,
        created_at,
    }
}
