use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use tauri::State;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::ai::tools::parse_reference_file;
use crate::db;
use crate::domain::models::{DeleteResult, EngagementFile, StoredEngagementFile, ToolPermission};
use crate::domain::rbac;
use crate::error::{AppError, AppResult};
use crate::security::{audit, crypto};
use crate::state::AppState;

const MAX_FILE_BYTES: usize = 25 * 1024 * 1024;

#[tauri::command]
pub async fn add_engagement_file(
    state: State<'_, AppState>,
    engagement_id: String,
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
    let bytes = BASE64
        .decode(data_base64.as_bytes())
        .map_err(|_| AppError::Validation("The file is not valid base64.".into()))?;
    if bytes.is_empty() || bytes.len() > MAX_FILE_BYTES {
        return Err(AppError::Validation(
            "Reference files must be between 1 byte and 25 MB.".into(),
        ));
    }
    let bytes = Zeroizing::new(bytes);
    let parsed = parse_reference_file(
        &filename,
        if mime_type.trim().is_empty() {
            "application/octet-stream"
        } else {
            mime_type.trim()
        },
        &bytes,
    )?;
    let data_dir = state.data_dir.clone();
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        db::test_cases::ensure_engagement(&session.conn, &engagement_id)?;
        let id = Uuid::now_v7().to_string();
        let now = db::now();
        let key = crypto::derive_evidence_key(&session.master_key, &engagement_id);
        let aad = format!("testtracker-reference-v1:{engagement_id}:{id}");
        let encrypted = crypto::encrypt(&key, &bytes, aad.as_bytes())?;
        let plaintext_hash = blake3::hash(&bytes).to_hex().to_string();
        let directory = files_root(&data_dir).join(&engagement_id);
        fs::create_dir_all(&directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
        }
        let path = directory.join(format!("{id}.bin"));
        write_private(&path, &encrypted.ciphertext)?;
        let file = EngagementFile {
            id: id.clone(),
            engagement_id: engagement_id.clone(),
            kind: parsed.kind,
            filename: filename.clone(),
            mime_type: if mime_type.trim().is_empty() {
                "application/octet-stream".into()
            } else {
                mime_type.trim().to_string()
            },
            size: bytes.len() as i64,
            summary: parsed.summary.clone(),
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
                    engagement_id: Some(&engagement_id),
                    actor_id: Some(&session.profile.id),
                    action: "engagement_file.added",
                    entity_type: "engagement_file",
                    entity_id: Some(&id),
                    metadata: serde_json::json!({
                        "filename": file.filename,
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
    })
}

#[tauri::command]
pub async fn list_engagement_files(
    state: State<'_, AppState>,
    engagement_id: String,
) -> AppResult<Vec<EngagementFile>> {
    state.with_session(|session| db::engagement_files::list(&session.conn, &engagement_id))
}

#[tauri::command]
pub async fn delete_engagement_file(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<DeleteResult> {
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

#[tauri::command]
pub async fn list_tool_permissions(
    state: State<'_, AppState>,
    engagement_id: String,
) -> AppResult<Vec<ToolPermission>> {
    state.with_session(|session| {
        db::engagement_files::tool_permissions(&session.conn, &engagement_id)
    })
}

#[tauri::command]
pub async fn revoke_tool_permission(
    state: State<'_, AppState>,
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
