use rusqlite::{Connection, OptionalExtension};

use crate::domain::models::{Snapshot, SnapshotSummary};
use crate::error::{AppError, AppResult};

pub fn insert(conn: &Connection, snapshot: &Snapshot) -> AppResult<()> {
    conn.execute(
        "INSERT INTO snapshots (
            id, engagement_id, version, label, snapshot_json, content_hash, created_by, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            snapshot.id,
            snapshot.engagement_id,
            snapshot.version,
            snapshot.label,
            snapshot.snapshot_json,
            snapshot.content_hash,
            snapshot.created_by,
            snapshot.created_at,
        ],
    )?;
    Ok(())
}

pub fn list(conn: &Connection, engagement_id: &str) -> AppResult<Vec<SnapshotSummary>> {
    let mut stmt = conn.prepare(
        "SELECT id, engagement_id, version, label, content_hash, created_by, created_at, snapshot_json
         FROM snapshots WHERE engagement_id = ?1 ORDER BY version DESC, created_at DESC",
    )?;
    let rows = stmt.query_map([engagement_id], |row| {
        let snapshot_json: String = row.get(7)?;
        let content_hash: String = row.get(4)?;
        Ok(SnapshotSummary {
            id: row.get(0)?,
            engagement_id: row.get(1)?,
            version: row.get(2)?,
            label: row.get(3)?,
            content_hash: content_hash.clone(),
            created_by: row.get(5)?,
            created_at: row.get(6)?,
            verified: blake3::hash(snapshot_json.as_bytes()).to_hex().to_string() == content_hash,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Snapshot> {
    let row = conn
        .query_row(
            "SELECT id, engagement_id, version, label, snapshot_json, content_hash, created_by, created_at
             FROM snapshots WHERE id = ?1",
            [id],
            |row| {
                let snapshot_json: String = row.get(4)?;
                let content_hash: String = row.get(5)?;
                Ok(Snapshot {
                    id: row.get(0)?,
                    engagement_id: row.get(1)?,
                    version: row.get(2)?,
                    label: row.get(3)?,
                    verified: blake3::hash(snapshot_json.as_bytes()).to_hex().to_string()
                        == content_hash,
                    snapshot_json,
                    content_hash,
                    created_by: row.get(6)?,
                    created_at: row.get(7)?,
                })
            },
        )
        .optional()?;
    row.ok_or(AppError::NotFound)
}

pub fn belongs_to_engagement(
    conn: &Connection,
    snapshot_id: &str,
    engagement_id: &str,
) -> AppResult<bool> {
    let found: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM snapshots WHERE id = ?1 AND engagement_id = ?2",
            rusqlite::params![snapshot_id, engagement_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(found.is_some())
}
