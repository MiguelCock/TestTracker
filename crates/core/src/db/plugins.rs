use rusqlite::{Connection, OptionalExtension};

use crate::error::{AppError, AppResult};
use crate::plugins::model::{PluginPublisher, PluginSummary};

pub struct StoredPlugin {
    pub id: String,
    pub version: String,
    pub name: String,
    pub publisher_id: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub manifest_json: String,
    pub wasm_path: String,
    pub wasm_hash: String,
    pub enabled: bool,
    pub installed_at: String,
}

pub fn insert_publisher(
    conn: &Connection,
    id: &str,
    name: &str,
    public_key: &str,
    created_by: &str,
    created_at: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO plugin_publishers (id, name, public_key, created_by, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, name, public_key, created_by, created_at],
    )?;
    Ok(())
}

pub fn list_publishers(conn: &Connection) -> AppResult<Vec<PluginPublisher>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, public_key, created_at FROM plugin_publishers ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(PluginPublisher {
            id: row.get(0)?,
            name: row.get(1)?,
            public_key: row.get(2)?,
            created_at: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get_publisher(conn: &Connection, id: &str) -> AppResult<PluginPublisher> {
    let row = conn
        .query_row(
            "SELECT id, name, public_key, created_at FROM plugin_publishers WHERE id = ?1",
            [id],
            |row| {
                Ok(PluginPublisher {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    public_key: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )
        .optional()?;
    row.ok_or(AppError::NotFound)
}

pub fn delete_publisher(conn: &Connection, id: &str) -> AppResult<()> {
    let changed = conn.execute("DELETE FROM plugin_publishers WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub fn upsert(
    conn: &Connection,
    plugin: &StoredPlugin,
    installed_by: &str,
    installed_at: &str,
) -> AppResult<()> {
    let capabilities = serde_json::to_string(&plugin.capabilities)?;
    conn.execute(
        "INSERT INTO plugins (
            id, version, name, publisher_id, description, capabilities_json,
            manifest_json, wasm_path, wasm_hash, enabled, installed_by, installed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO UPDATE SET
            version = excluded.version,
            name = excluded.name,
            publisher_id = excluded.publisher_id,
            description = excluded.description,
            capabilities_json = excluded.capabilities_json,
            manifest_json = excluded.manifest_json,
            wasm_path = excluded.wasm_path,
            wasm_hash = excluded.wasm_hash,
            enabled = excluded.enabled,
            installed_by = excluded.installed_by,
            installed_at = excluded.installed_at",
        rusqlite::params![
            plugin.id,
            plugin.version,
            plugin.name,
            plugin.publisher_id,
            plugin.description,
            capabilities,
            plugin.manifest_json,
            plugin.wasm_path,
            plugin.wasm_hash,
            if plugin.enabled { 1 } else { 0 },
            installed_by,
            installed_at,
        ],
    )?;
    Ok(())
}

pub fn list(conn: &Connection) -> AppResult<Vec<PluginSummary>> {
    let mut stmt = conn.prepare(
        "SELECT id, version, name, publisher_id, description, capabilities_json,
                manifest_json, wasm_path, wasm_hash, enabled, installed_at
         FROM plugins ORDER BY installed_at DESC",
    )?;
    let rows = stmt.query_map([], map_row)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(to_stored(row?)?.summary());
    }
    Ok(out)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<StoredPlugin> {
    let row = conn
        .query_row(
            "SELECT id, version, name, publisher_id, description, capabilities_json,
                    manifest_json, wasm_path, wasm_hash, enabled, installed_at
             FROM plugins WHERE id = ?1",
            [id],
            map_row,
        )
        .optional()?
        .ok_or(AppError::NotFound)?;
    to_stored(row)
}

pub fn set_enabled(conn: &Connection, id: &str, enabled: bool) -> AppResult<()> {
    let changed = conn.execute(
        "UPDATE plugins SET enabled = ?1 WHERE id = ?2",
        rusqlite::params![if enabled { 1 } else { 0 }, id],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let changed = conn.execute("DELETE FROM plugins WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

type PluginRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
    String,
);

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PluginRow> {
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

fn to_stored(row: PluginRow) -> AppResult<StoredPlugin> {
    let (
        id,
        version,
        name,
        publisher_id,
        description,
        capabilities_json,
        manifest_json,
        wasm_path,
        wasm_hash,
        enabled,
        installed_at,
    ) = row;
    let capabilities: Vec<String> = serde_json::from_str(&capabilities_json).unwrap_or_default();
    Ok(StoredPlugin {
        id,
        version,
        name,
        publisher_id,
        description,
        capabilities,
        manifest_json,
        wasm_path,
        wasm_hash,
        enabled: enabled != 0,
        installed_at,
    })
}

impl StoredPlugin {
    pub fn summary(&self) -> PluginSummary {
        PluginSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            version: self.version.clone(),
            publisher_id: self.publisher_id.clone(),
            description: self.description.clone(),
            capabilities: self.capabilities.clone(),
            enabled: self.enabled,
            installed_at: self.installed_at.clone(),
        }
    }
}
