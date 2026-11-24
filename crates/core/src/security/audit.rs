use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::error::AppResult;

pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Serialize)]
struct AuditPayload<'a> {
    seq: i64,
    id: &'a str,
    engagement_id: Option<&'a str>,
    actor_id: Option<&'a str>,
    action: &'a str,
    entity_type: &'a str,
    entity_id: Option<&'a str>,
    metadata_json: &'a str,
    prev_hash: &'a str,
    created_at: &'a str,
}

pub struct AuditEntry<'a> {
    pub engagement_id: Option<&'a str>,
    pub actor_id: Option<&'a str>,
    pub action: &'a str,
    pub entity_type: &'a str,
    pub entity_id: Option<&'a str>,
    pub metadata: serde_json::Value,
    pub created_at: &'a str,
}

pub fn append(conn: &Connection, entry: AuditEntry<'_>) -> AppResult<String> {
    let prev_hash: String = conn
        .query_row(
            "SELECT hash FROM audit_events ORDER BY seq DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or_else(|| GENESIS_HASH.to_string());

    let seq: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM audit_events",
            [],
            |row| row.get(0),
        )
        .unwrap_or(1);

    let id = uuid::Uuid::now_v7().to_string();
    let metadata_json = serde_json::to_string(&entry.metadata)?;
    let payload = AuditPayload {
        seq,
        id: &id,
        engagement_id: entry.engagement_id,
        actor_id: entry.actor_id,
        action: entry.action,
        entity_type: entry.entity_type,
        entity_id: entry.entity_id,
        metadata_json: &metadata_json,
        prev_hash: &prev_hash,
        created_at: entry.created_at,
    };
    let bytes = serde_json::to_vec(&payload)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(prev_hash.as_bytes());
    hasher.update(&bytes);
    let hash = hasher.finalize().to_hex().to_string();

    conn.execute(
        "INSERT INTO audit_events (
            seq, id, engagement_id, actor_id, action, entity_type, entity_id,
            metadata_json, prev_hash, hash, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        rusqlite::params![
            seq,
            id,
            entry.engagement_id,
            entry.actor_id,
            entry.action,
            entry.entity_type,
            entry.entity_id,
            metadata_json,
            prev_hash,
            hash,
            entry.created_at,
        ],
    )?;

    Ok(hash)
}

pub fn verify_chain(conn: &Connection) -> AppResult<bool> {
    let mut stmt = conn.prepare(
        "SELECT seq, id, engagement_id, actor_id, action, entity_type, entity_id,
                metadata_json, prev_hash, hash, created_at
         FROM audit_events ORDER BY seq ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, String>(10)?,
        ))
    })?;

    let mut expected_prev = GENESIS_HASH.to_string();
    for row in rows {
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
        ) = row?;
        if prev_hash != expected_prev {
            return Ok(false);
        }
        let payload = AuditPayload {
            seq,
            id: &id,
            engagement_id: engagement_id.as_deref(),
            actor_id: actor_id.as_deref(),
            action: &action,
            entity_type: &entity_type,
            entity_id: entity_id.as_deref(),
            metadata_json: &metadata_json,
            prev_hash: &prev_hash,
            created_at: &created_at,
        };
        let bytes = serde_json::to_vec(&payload)?;
        let mut hasher = blake3::Hasher::new();
        hasher.update(prev_hash.as_bytes());
        hasher.update(&bytes);
        let computed = hasher.finalize().to_hex().to_string();
        if computed != hash {
            return Ok(false);
        }
        expected_prev = hash;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn conn() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open");
        db::apply_migrations(&mut conn).expect("migrate");
        conn
    }

    #[test]
    fn chain_verifies_and_detects_tampering() {
        let conn = conn();
        append(
            &conn,
            AuditEntry {
                engagement_id: None,
                actor_id: None,
                action: "test.created",
                entity_type: "test",
                entity_id: Some("1"),
                metadata: serde_json::json!({ "a": 1 }),
                created_at: "2026-01-01T00:00:00Z",
            },
        )
        .expect("append");
        append(
            &conn,
            AuditEntry {
                engagement_id: None,
                actor_id: None,
                action: "test.updated",
                entity_type: "test",
                entity_id: Some("1"),
                metadata: serde_json::json!({ "a": 2 }),
                created_at: "2026-01-01T00:00:01Z",
            },
        )
        .expect("append");
        assert!(verify_chain(&conn).expect("verify"));

        conn.execute(
            "UPDATE audit_events SET action = 'tampered' WHERE seq = 1",
            [],
        )
        .expect("update");
        assert!(!verify_chain(&conn).expect("verify"));
    }

    #[test]
    fn chain_links_previous_hash() {
        let conn = conn();
        append(
            &conn,
            AuditEntry {
                engagement_id: None,
                actor_id: None,
                action: "one",
                entity_type: "test",
                entity_id: None,
                metadata: serde_json::json!({}),
                created_at: "2026-01-01T00:00:00Z",
            },
        )
        .expect("append");
        let first: String = conn
            .query_row("SELECT hash FROM audit_events WHERE seq = 1", [], |row| {
                row.get(0)
            })
            .expect("hash");
        append(
            &conn,
            AuditEntry {
                engagement_id: None,
                actor_id: None,
                action: "two",
                entity_type: "test",
                entity_id: None,
                metadata: serde_json::json!({}),
                created_at: "2026-01-01T00:00:01Z",
            },
        )
        .expect("append");
        let prev: String = conn
            .query_row(
                "SELECT prev_hash FROM audit_events WHERE seq = 2",
                [],
                |row| row.get(0),
            )
            .expect("prev");
        assert_eq!(prev, first);
    }
}
