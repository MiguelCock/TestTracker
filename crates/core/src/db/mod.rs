pub mod ai;
pub mod approvals;
pub mod audit_repo;
pub mod engagement_files;
pub mod engagements;
pub mod evidence;
pub mod findings;
pub mod generated_documents;
pub mod metrics;
pub mod plugins;
pub mod profiles;
pub mod schema;
pub mod seed;
pub mod settings;
pub mod snapshots;
pub mod templates;
pub mod test_cases;

#[cfg(test)]
mod migration_tests;

use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension};

use crate::error::{AppError, AppResult};
use crate::security::crypto::derive_db_key;

pub fn open_encrypted(path: &Path, master_key: &[u8; 32]) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if path.exists() {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
    }
    let cipher_version: Option<String> = conn
        .query_row("PRAGMA cipher_version", [], |row| row.get(0))
        .optional()?;
    if cipher_version.is_none() {
        return Err(AppError::Internal(
            "SQLCipher support is not available in this build.".into(),
        ));
    }

    let db_key = derive_db_key(master_key);
    let key_hex = hex::encode(db_key);
    conn.execute_batch(&format!("PRAGMA key = \"x'{key_hex}'\";"))?;
    conn.pragma_update(None, "cipher_memory_security", "ON")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.busy_timeout(Duration::from_secs(5))?;

    conn.query_row("SELECT COUNT(*) FROM sqlite_master", [], |_| Ok(()))?;
    Ok(conn)
}

pub fn apply_migrations(conn: &mut Connection) -> AppResult<()> {
    let mut version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > schema::LATEST_VERSION {
        return Err(AppError::Conflict(format!(
            "database schema version {version} is newer than this build supports ({}).",
            schema::LATEST_VERSION
        )));
    }
    while version < schema::LATEST_VERSION {
        let index = usize::try_from(version)
            .map_err(|_| AppError::Internal("invalid database schema version".into()))?;
        let migration = schema::MIGRATIONS.get(index).ok_or_else(|| {
            AppError::Internal(format!(
                "missing migration {index} for database schema version {version}"
            ))
        })?;
        let tx = conn.transaction()?;
        tx.execute_batch(migration)?;
        tx.pragma_update(None, "user_version", version + 1)?;
        tx.commit()?;
        version += 1;
    }
    let tx = conn.transaction()?;
    seed::seed_templates(&tx)?;
    tx.commit()?;
    Ok(())
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
