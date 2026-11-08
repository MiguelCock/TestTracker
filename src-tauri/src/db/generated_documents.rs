use rusqlite::{Connection, OptionalExtension};

use crate::documents::model::{DocumentFormat, DocumentKind, GeneratedDocument};
use crate::error::{AppError, AppResult};

pub struct StoredDocument {
    pub document: GeneratedDocument,
    pub nonce: String,
    pub path: String,
}

pub fn insert(conn: &Connection, stored: &StoredDocument) -> AppResult<()> {
    let document = &stored.document;
    conn.execute(
        "INSERT INTO generated_documents (
            id, engagement_id, kind, format, title, filename, size, content_hash,
            nonce, path, created_by, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        rusqlite::params![
            document.id,
            document.engagement_id,
            document.kind.as_str(),
            document.format.as_str(),
            document.title,
            document.filename,
            document.size,
            document.content_hash,
            stored.nonce,
            stored.path,
            document.created_by,
            document.created_at,
        ],
    )?;
    Ok(())
}

pub fn list(conn: &Connection, engagement_id: &str) -> AppResult<Vec<GeneratedDocument>> {
    let mut stmt = conn.prepare(
        "SELECT id, engagement_id, kind, format, title, filename, size, content_hash,
                created_by, created_at
         FROM generated_documents WHERE engagement_id = ?1 ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([engagement_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (
            id,
            engagement_id,
            kind,
            format,
            title,
            filename,
            size,
            content_hash,
            created_by,
            created_at,
        ) = row?;
        out.push(GeneratedDocument {
            id,
            engagement_id,
            kind: DocumentKind::from_db(&kind)?,
            format: DocumentFormat::from_db(&format)?,
            title,
            filename,
            size,
            content_hash,
            created_by,
            created_at,
        });
    }
    Ok(out)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<StoredDocument> {
    let row = conn
        .query_row(
            "SELECT id, engagement_id, kind, format, title, filename, size, content_hash,
                    created_by, created_at, nonce, path
             FROM generated_documents WHERE id = ?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, String>(11)?,
                ))
            },
        )
        .optional()?;
    let Some((
        id,
        engagement_id,
        kind,
        format,
        title,
        filename,
        size,
        content_hash,
        created_by,
        created_at,
        nonce,
        path,
    )) = row
    else {
        return Err(AppError::NotFound);
    };
    Ok(StoredDocument {
        document: GeneratedDocument {
            id,
            engagement_id,
            kind: DocumentKind::from_db(&kind)?,
            format: DocumentFormat::from_db(&format)?,
            title,
            filename,
            size,
            content_hash,
            created_by,
            created_at,
        },
        nonce,
        path,
    })
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let changed = conn.execute("DELETE FROM generated_documents WHERE id = ?1", [id])?;
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}
