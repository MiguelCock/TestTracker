use rusqlite::Connection;

use crate::domain::models::AuditEvent;
use crate::error::AppResult;

const COLUMNS: &str = "seq, id, engagement_id, actor_id, action, entity_type, entity_id,
                       metadata_json, prev_hash, hash, created_at";

type AuditRow = (
    i64,
    String,
    Option<String>,
    Option<String>,
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
);

pub fn list(
    conn: &Connection,
    engagement_id: Option<&str>,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<AuditEvent>> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    let mut out = Vec::new();
    if let Some(id) = engagement_id {
        let sql = format!(
            "SELECT {COLUMNS} FROM audit_events WHERE engagement_id = ?1 ORDER BY seq DESC LIMIT ?2 OFFSET ?3"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params![id, limit, offset], map_row)?;
        for row in rows {
            out.push(to_event(row?));
        }
    } else {
        let sql =
            format!("SELECT {COLUMNS} FROM audit_events ORDER BY seq DESC LIMIT ?1 OFFSET ?2");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params![limit, offset], map_row)?;
        for row in rows {
            out.push(to_event(row?));
        }
    }
    Ok(out)
}

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuditRow> {
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
    ))
}

fn to_event(row: AuditRow) -> AuditEvent {
    let (
        seq,
        id,
        engagement_id,
        actor_id,
        action,
        entity_type,
        entity_id,
        metadata_json,
        prev_hash,
        hash,
        created_at,
    ) = row;
    AuditEvent {
        seq,
        id,
        engagement_id,
        actor_id,
        action,
        entity_type,
        entity_id,
        metadata: serde_json::from_str(&metadata_json).unwrap_or(serde_json::json!({})),
        prev_hash,
        hash,
        created_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::security::audit;

    #[test]
    fn paginates_events_in_descending_sequence() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("testtracker.db");
        let master = crate::security::crypto::random_bytes::<32>();
        let mut conn = db::open_encrypted(&path, &master).expect("open");
        db::apply_migrations(&mut conn).expect("migrate");
        for index in 0..3 {
            audit::append(
                &conn,
                audit::AuditEntry {
                    engagement_id: None,
                    actor_id: None,
                    action: "test.event",
                    entity_type: "test",
                    entity_id: None,
                    metadata: serde_json::json!({ "index": index }),
                    created_at: "2026-01-01T00:00:00Z",
                },
            )
            .expect("append");
        }

        let first = list(&conn, None, 2, 0).expect("first page");
        assert_eq!(first.len(), 2);
        assert!(first[0].seq > first[1].seq);

        let second = list(&conn, None, 2, 2).expect("second page");
        assert_eq!(second.len(), 1);
        assert!(second[0].seq < first[1].seq);
    }
}
