use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::ai::tools::{ParsedReferenceFile, parse_reference_file};
use crate::db;
use crate::domain::models::{
    DeleteResult, EngagementFile, EngagementFileData, EngagementFilePreview, StoredEngagementFile,
    ToolPermission,
};
use crate::domain::rbac;
use crate::error::{AppError, AppResult};
use crate::security::{audit, crypto};
use crate::state::AppState;

const MAX_FILE_BYTES: usize = 25 * 1024 * 1024;

pub async fn add_engagement_file(
    state: Arc<AppState>,
    engagement_id: String,
    relative_path: String,
    filename: String,
    mime_type: String,
    data_base64: String,
) -> AppResult<EngagementFile> {
    let filename = filename.trim().to_string();
    if filename.is_empty() || filename.chars().count() > 255 {
        return Err(AppError::Validation(
            "A valid file name is required.".into(),
        ));
    }
    let relative_path = normalize_relative_path(&relative_path, &filename)?;
    let bytes = BASE64
        .decode(data_base64.as_bytes())
        .map_err(|_| AppError::Validation("The file is not valid base64.".into()))?;
    if bytes.is_empty() || bytes.len() > MAX_FILE_BYTES {
        return Err(AppError::Validation(
            "Reference files must be between 1 byte and 25 MB.".into(),
        ));
    }
    let bytes = Zeroizing::new(bytes);
    let data_dir = state.data_dir.clone();
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        db::test_cases::ensure_engagement(&session.conn, &engagement_id)?;
        store_bytes(
            session,
            &data_dir,
            &engagement_id,
            &relative_path,
            &filename,
            if mime_type.trim().is_empty() {
                "application/octet-stream"
            } else {
                mime_type.trim()
            },
            &bytes,
            None,
            "file.added",
        )
    })
}

#[allow(clippy::too_many_arguments)]
fn store_bytes(
    session: &mut crate::state::UnlockedSession,
    data_dir: &Path,
    engagement_id: &str,
    relative_path: &str,
    filename: &str,
    mime_type: &str,
    bytes: &[u8],
    summary_extra: Option<serde_json::Value>,
    audit_action: &str,
) -> AppResult<EngagementFile> {
    let parsed = parse_storage_file(filename, mime_type, bytes)?;
    let id = Uuid::now_v7().to_string();
    let now = db::now();
    let key = crypto::derive_evidence_key(&session.master_key, engagement_id);
    let aad = format!("testtracker-reference-v1:{engagement_id}:{id}");
    let encrypted = crypto::encrypt(&key, bytes, aad.as_bytes())?;
    let plaintext_hash = blake3::hash(bytes).to_hex().to_string();
    let directory = files_root(data_dir).join(engagement_id);
    fs::create_dir_all(&directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    }
    let path = directory.join(format!("{id}.bin"));
    write_private(&path, &encrypted.ciphertext)?;
    let mut summary = parsed.summary.clone();
    if let (Some(extra), Some(target)) = (summary_extra, summary.as_object_mut())
        && let Some(object) = extra.as_object()
    {
        for (key, value) in object {
            target.insert(key.clone(), value.clone());
        }
    }
    let file = EngagementFile {
        id: id.clone(),
        engagement_id: engagement_id.to_string(),
        relative_path: relative_path.to_string(),
        kind: parsed.kind,
        filename: filename.to_string(),
        mime_type: mime_type.to_string(),
        size: bytes.len() as i64,
        summary: summary.clone(),
        created_by: session.profile.id.clone(),
        created_at: now.clone(),
    };
    let stored = StoredEngagementFile {
        file: file.clone(),
        plaintext_hash: plaintext_hash.clone(),
        nonce: hex::encode(encrypted.nonce),
        path: path.to_string_lossy().to_string(),
        extracted_text: parsed.text,
    };
    let result = (|| -> AppResult<EngagementFile> {
        let tx = session.conn.transaction()?;
        db::engagement_files::insert(&tx, &stored)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(engagement_id),
                actor_id: Some(&session.profile.id),
                action: audit_action,
                entity_type: "engagement_file",
                entity_id: Some(&id),
                metadata: serde_json::json!({
                    "filename": file.filename,
                    "path": file.relative_path,
                    "kind": file.kind,
                    "size": file.size,
                    "hash": plaintext_hash,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(file)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&path);
    }
    result
}

pub async fn migrate_evidence_to_files(state: Arc<AppState>) -> AppResult<usize> {
    let data_dir = state.data_dir.clone();
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let engagements = db::engagements::list(&session.conn)?;
        let mut migrated = 0usize;
        for engagement in engagements {
            let evidence = db::evidence::list(&session.conn, &engagement.id)?;
            if evidence.is_empty() {
                continue;
            }
            let existing = db::engagement_files::list(&session.conn, &engagement.id)?;
            for item in evidence {
                let already = existing.iter().any(|file| {
                    file.summary
                        .get("migratedFromEvidence")
                        .and_then(|value| value.as_str())
                        == Some(item.id.as_str())
                });
                if already {
                    continue;
                }
                let (evidence, bytes) =
                    crate::services::evidence::read_evidence_content(&data_dir, session, &item.id)?;
                let path = format!("evidence/{}", evidence.filename);
                store_bytes(
                    session,
                    &data_dir,
                    &engagement.id,
                    &path,
                    &evidence.filename,
                    &evidence.mime_type,
                    &bytes,
                    Some(serde_json::json!({
                        "migratedFromEvidence": evidence.id,
                        "caption": evidence.caption,
                    })),
                    "file.migrated_from_evidence",
                )?;
                let now = db::now();
                let tx = session.conn.transaction()?;
                db::evidence::delete(&tx, &evidence.id)?;
                db::evidence::delete_blob(&tx, &evidence.blob_id)?;
                audit::append(
                    &tx,
                    audit::AuditEntry {
                        engagement_id: Some(&engagement.id),
                        actor_id: Some(&session.profile.id),
                        action: "evidence.migrated",
                        entity_type: "evidence",
                        entity_id: Some(&evidence.id),
                        metadata: serde_json::json!({ "path": path }),
                        created_at: &now,
                    },
                )?;
                tx.commit()?;
                migrated += 1;
            }
        }
        Ok(migrated)
    })
}

pub async fn list_engagement_files(
    state: Arc<AppState>,
    engagement_id: String,
) -> AppResult<Vec<EngagementFile>> {
    state.with_session(|session| db::engagement_files::list(&session.conn, &engagement_id))
}

pub async fn get_engagement_file_preview(
    state: Arc<AppState>,
    id: String,
) -> AppResult<EngagementFilePreview> {
    state.with_session(|session| {
        let stored = db::engagement_files::get(&session.conn, &id)?;
        Ok(EngagementFilePreview {
            file: stored.file,
            text: stored.extracted_text.chars().take(200_000).collect(),
        })
    })
}

pub async fn get_engagement_file_data(
    state: Arc<AppState>,
    id: String,
) -> AppResult<EngagementFileData> {
    let data_dir = state.data_dir.clone();
    state.with_session(|session| {
        let stored = db::engagement_files::get(&session.conn, &id)?;
        let path = validate_path(&data_dir, &stored.path)?;
        let encrypted = fs::read(path)?;
        let key = crypto::derive_evidence_key(&session.master_key, &stored.file.engagement_id);
        let aad = format!(
            "testtracker-reference-v1:{}:{}",
            stored.file.engagement_id, stored.file.id
        );
        let plaintext = crypto::decrypt(
            &key,
            &encrypted,
            &hex::decode(&stored.nonce).map_err(|_| AppError::Crypto)?,
            aad.as_bytes(),
        )?;
        let hash = blake3::hash(&plaintext).to_hex().to_string();
        if hash != stored.plaintext_hash {
            return Err(AppError::Crypto);
        }
        Ok(EngagementFileData {
            file: stored.file,
            data_base64: BASE64.encode(&plaintext),
        })
    })
}

pub async fn delete_engagement_file(state: Arc<AppState>, id: String) -> AppResult<DeleteResult> {
    let data_dir = state.data_dir.clone();
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let stored = db::engagement_files::get(&session.conn, &id)?;
        let path = validate_path(&data_dir, &stored.path)?;
        let now = db::now();
        let tx = session.conn.transaction()?;
        db::engagement_files::delete(&tx, &id)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&stored.file.engagement_id),
                actor_id: Some(&session.profile.id),
                action: "engagement_file.deleted",
                entity_type: "engagement_file",
                entity_id: Some(&id),
                metadata: serde_json::json!({
                    "filename": stored.file.filename,
                    "kind": stored.file.kind,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        let _ = fs::remove_file(path);
        Ok(DeleteResult { id })
    })
}

pub async fn list_tool_permissions(
    state: Arc<AppState>,
    engagement_id: String,
) -> AppResult<Vec<ToolPermission>> {
    state.with_session(|session| {
        db::engagement_files::tool_permissions(&session.conn, &engagement_id)
    })
}

pub async fn revoke_tool_permission(
    state: Arc<AppState>,
    engagement_id: String,
    tool: String,
) -> AppResult<Vec<ToolPermission>> {
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let now = db::now();
        let tx = session.conn.transaction()?;
        db::engagement_files::revoke_tool(&tx, &engagement_id, &tool)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&engagement_id),
                actor_id: Some(&session.profile.id),
                action: "tool_permission.revoked",
                entity_type: "tool_permission",
                entity_id: Some(&tool),
                metadata: serde_json::json!({ "tool": tool }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        db::engagement_files::tool_permissions(&session.conn, &engagement_id)
    })
}

fn normalize_relative_path(relative_path: &str, filename: &str) -> AppResult<String> {
    let trimmed = relative_path.trim();
    if trimmed.starts_with('/') || trimmed.contains('\\') {
        return Err(AppError::Validation("Invalid folder path.".into()));
    }
    let raw = trimmed.trim_matches('/');
    let candidate = if raw.is_empty() {
        filename.to_string()
    } else {
        format!("{raw}/{filename}")
    };
    if candidate.len() > 500 || candidate.starts_with('/') || candidate.contains('\\') {
        return Err(AppError::Validation("Invalid folder path.".into()));
    }
    let mut parts = Vec::new();
    for part in candidate.split('/') {
        let part = part.trim();
        if part.is_empty() || part == "." || part == ".." {
            return Err(AppError::Validation("Invalid folder path.".into()));
        }
        if part.chars().count() > 120 {
            return Err(AppError::Validation("Folder name is too long.".into()));
        }
        parts.push(part);
    }
    if parts.len() > 12 {
        return Err(AppError::Validation("Folder nesting is too deep.".into()));
    }
    Ok(parts.join("/"))
}

pub(crate) fn parse_storage_file(
    filename: &str,
    mime_type: &str,
    bytes: &[u8],
) -> AppResult<ParsedReferenceFile> {
    match parse_reference_file(filename, mime_type, bytes) {
        Ok(parsed) => Ok(parsed),
        Err(AppError::Validation(_)) => {
            let extension = filename
                .rsplit_once('.')
                .map(|(_, extension)| extension.to_ascii_lowercase())
                .unwrap_or_default();
            let kind = if mime_type.starts_with("image/") {
                "image"
            } else if mime_type == "application/pdf" || extension == "pdf" {
                "pdf"
            } else if mime_type.starts_with("text/") {
                "text"
            } else {
                "binary"
            };
            Ok(ParsedReferenceFile {
                kind: kind.to_string(),
                summary: serde_json::json!({
                    "mimeType": mime_type,
                    "size": bytes.len(),
                    "note": "Binary or unsupported file stored without text extraction.",
                }),
                text: String::new(),
            })
        }
        Err(error) => Err(error),
    }
}

fn files_root(data_dir: &Path) -> PathBuf {
    data_dir.join("engagement-files")
}

fn validate_path(data_dir: &Path, stored: &str) -> AppResult<PathBuf> {
    let path = PathBuf::from(stored);
    if !path.starts_with(files_root(data_dir)) {
        return Err(AppError::Internal("invalid reference file path".into()));
    }
    Ok(path)
}

fn write_private(path: &Path, contents: &[u8]) -> AppResult<()> {
    let tmp = path.with_extension("tmp");
    let mut options = fs::OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&tmp)?;
    file.write_all(contents)?;
    file.sync_all()?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_virtual_paths() {
        assert_eq!(
            normalize_relative_path("postman", "api.json").expect("path"),
            "postman/api.json"
        );
        assert_eq!(
            normalize_relative_path("postman/", "api.json").expect("path"),
            "postman/api.json"
        );
        assert_eq!(
            normalize_relative_path("", "api.json").expect("path"),
            "api.json"
        );
    }

    #[test]
    fn rejects_traversal_and_absolute_paths() {
        assert!(normalize_relative_path("../etc", "passwd").is_err());
        assert!(normalize_relative_path("/etc", "passwd").is_err());
        assert!(normalize_relative_path("a//b", "file").is_err());
    }

    #[test]
    fn binary_files_get_a_metadata_only_fallback() {
        let parsed = parse_storage_file("screenshot.png", "image/png", &[0, 1, 2, 3])
            .expect("binary fallback");
        assert_eq!(parsed.kind, "image");
        assert!(parsed.text.is_empty());
    }
}
