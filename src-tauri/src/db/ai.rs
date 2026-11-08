use rusqlite::{Connection, OptionalExtension};

use crate::ai::model::{AiPolicy, AiProviderKind};
use crate::error::{AppError, AppResult};

pub struct StoredAiProvider {
    pub id: String,
    pub name: String,
    pub kind: AiProviderKind,
    pub base_url: String,
    pub model: String,
    pub encrypted_api_key: String,
    pub created_by: String,
    pub created_at: String,
}

pub fn insert(conn: &Connection, provider: &StoredAiProvider) -> AppResult<()> {
    conn.execute(
        "INSERT INTO ai_providers (
            id, name, kind, base_url, model, encrypted_api_key, created_by, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            provider.id,
            provider.name,
            provider.kind.as_str(),
            provider.base_url,
            provider.model,
            provider.encrypted_api_key,
            provider.created_by,
            provider.created_at,
        ],
    )?;
    Ok(())
}

pub fn list(conn: &Connection) -> AppResult<Vec<StoredAiProvider>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, kind, base_url, model, encrypted_api_key, created_by, created_at
         FROM ai_providers ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map([], map_row)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(to_stored(row?)?);
    }
    Ok(out)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<StoredAiProvider> {
    let row = conn
        .query_row(
            "SELECT id, name, kind, base_url, model, encrypted_api_key, created_by, created_at
             FROM ai_providers WHERE id = ?1",
            [id],
            map_row,
        )
        .optional()?
        .ok_or(AppError::NotFound)?;
    to_stored(row)
}

pub fn update(conn: &Connection, provider: &StoredAiProvider) -> AppResult<()> {
    let changed = conn.execute(
        "UPDATE ai_providers SET
            name = ?1, kind = ?2, base_url = ?3, model = ?4, encrypted_api_key = ?5
         WHERE id = ?6",
        rusqlite::params![
            provider.name,
            provider.kind.as_str(),
            provider.base_url,
            provider.model,
            provider.encrypted_api_key,
            provider.id,
        ],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let changed = conn.execute("DELETE FROM ai_providers WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub fn get_policy(conn: &Connection, engagement_id: &str) -> AppResult<AiPolicy> {
    let row: Option<String> = conn
        .query_row(
            "SELECT policy FROM ai_policies WHERE engagement_id = ?1",
            [engagement_id],
            |row| row.get(0),
        )
        .optional()?;
    match row {
        Some(policy) => AiPolicy::from_db(&policy),
        None => Ok(AiPolicy::default()),
    }
}

pub fn set_policy(
    conn: &Connection,
    engagement_id: &str,
    policy: AiPolicy,
    actor: &str,
    now: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO ai_policies (engagement_id, policy, updated_by, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(engagement_id) DO UPDATE SET
            policy = excluded.policy,
            updated_by = excluded.updated_by,
            updated_at = excluded.updated_at",
        rusqlite::params![engagement_id, policy.as_str(), actor, now],
    )?;
    Ok(())
}

type ProviderRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProviderRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
    ))
}

fn to_stored(row: ProviderRow) -> AppResult<StoredAiProvider> {
    let (id, name, kind, base_url, model, encrypted_api_key, created_by, created_at) = row;
    Ok(StoredAiProvider {
        id,
        name,
        kind: AiProviderKind::from_db(&kind)?,
        base_url,
        model,
        encrypted_api_key,
        created_by,
        created_at,
    })
}
