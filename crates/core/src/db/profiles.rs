use rusqlite::{Connection, OptionalExtension};

use crate::domain::enums::ProfileRole;
use crate::error::AppResult;

pub struct StoredProfile {
    pub id: String,
    pub display_name: String,
    pub role: ProfileRole,
    pub public_key: String,
    pub encrypted_private_key: String,
}

pub fn insert(
    conn: &Connection,
    id: &str,
    display_name: &str,
    role: ProfileRole,
    public_key: &str,
    encrypted_private_key: &str,
    created_at: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO profiles (id, display_name, role, public_key, encrypted_private_key, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            id,
            display_name,
            role.as_str(),
            public_key,
            encrypted_private_key,
            created_at
        ],
    )?;
    Ok(())
}

pub fn first(conn: &Connection) -> AppResult<Option<StoredProfile>> {
    let row = conn
        .query_row(
            "SELECT id, display_name, role, public_key, encrypted_private_key
             FROM profiles ORDER BY created_at ASC LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?;
    row.map(to_stored).transpose()
}

pub fn get(conn: &Connection, id: &str) -> AppResult<StoredProfile> {
    let row = conn
        .query_row(
            "SELECT id, display_name, role, public_key, encrypted_private_key
             FROM profiles WHERE id = ?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?;
    row.map(to_stored)
        .transpose()?
        .ok_or(crate::error::AppError::NotFound)
}

fn to_stored(row: (String, String, String, String, String)) -> AppResult<StoredProfile> {
    let (id, display_name, role, public_key, encrypted_private_key) = row;
    Ok(StoredProfile {
        id,
        display_name,
        role: ProfileRole::from_db(&role)?,
        public_key,
        encrypted_private_key,
    })
}
