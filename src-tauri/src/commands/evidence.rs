use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use tauri::State;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::db;
use crate::domain::models::{DeleteResult, Evidence, EvidenceData, EvidenceInput};
use crate::domain::rbac;
use crate::error::{AppError, AppResult};
use crate::security::{audit, crypto};
use crate::state::{AppState, UnlockedSession};

const MAX_EVIDENCE_BYTES: usize = 8 * 1024 * 1024;

#[tauri::command]
pub async fn list_evidence(
    state: State<'_, AppState>,
    engagement_id: String,
) -> AppResult<Vec<Evidence>> {
    state.with_session(|session| db::evidence::list(&session.conn, &engagement_id))
}

#[tauri::command]
pub async fn add_evidence(state: State<'_, AppState>, input: EvidenceInput) -> AppResult<Evidence> {
    validate_input(&input)?;
    let data = BASE64
        .decode(input.data_base64.as_bytes())
        .map_err(|_| AppError::Validation("Evidence data is not valid base64.".into()))?;
    if data.is_empty() {
        return Err(AppError::Validation("Evidence file is empty.".into()));
    }
    if data.len() > MAX_EVIDENCE_BYTES {
        return Err(AppError::Validation(
            "Evidence files must be 8 MB or smaller.".into(),
        ));
    }
    let data = Zeroizing::new(data);
    let data_dir = state.data_dir.clone();

    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        db::test_cases::ensure_engagement(&session.conn, &input.engagement_id)?;
        if let Some(finding_id) = input.finding_id.as_deref()
            && !db::evidence::link_belongs_to_engagement(
                &session.conn,
                "findings",
                finding_id,
                &input.engagement_id,
            )?
        {
            return Err(AppError::Validation(
                "The linked finding does not belong to this engagement.".into(),
            ));
        }
        if let Some(test_case_id) = input.test_case_id.as_deref()
            && !db::evidence::link_belongs_to_engagement(
                &session.conn,
                "test_cases",
                test_case_id,
                &input.engagement_id,
            )?
        {
            return Err(AppError::Validation(
                "The linked test case does not belong to this engagement.".into(),
            ));
        }

        let actor = session.profile.id.clone();
        let now = db::now();
        let evidence_id = Uuid::now_v7().to_string();
        let blob_id = Uuid::now_v7().to_string();
        let key = crypto::derive_evidence_key(&session.master_key, &input.engagement_id);
        let aad = format!(
            "testtracker-evidence-v1:{}:{}",
            input.engagement_id, evidence_id
        );
        let encrypted = crypto::encrypt(&key, &data, aad.as_bytes())?;
        let plaintext_hash = blake3::hash(&data).to_hex().to_string();

        let directory = evidence_root(&data_dir).join(&input.engagement_id);
        fs::create_dir_all(&directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
        }
        let blob_path = directory.join(format!("{blob_id}.bin"));
        write_private(&blob_path, &encrypted.ciphertext)?;

        let evidence = Evidence {
            id: evidence_id.clone(),
            engagement_id: input.engagement_id.clone(),
            finding_id: input.finding_id.clone(),
            test_case_id: input.test_case_id.clone(),
            filename: input.filename.trim().to_string(),
            mime_type: if input.mime_type.trim().is_empty() {
                "application/octet-stream".to_string()
            } else {
                input.mime_type.trim().to_string()
            },
            size: data.len() as i64,
            blob_id: blob_id.clone(),
            plaintext_hash,
            caption: input.caption.trim().to_string(),
            created_by: actor.clone(),
            created_at: now.clone(),
        };

        let result = (|| -> AppResult<Evidence> {
            let tx = session.conn.transaction()?;
            db::evidence::insert_blob(
                &tx,
                &blob_id,
                &input.engagement_id,
                &hex::encode(encrypted.nonce),
                &blob_path.to_string_lossy(),
                &now,
            )?;
            db::evidence::insert(&tx, &evidence)?;
            audit::append(
                &tx,
                audit::AuditEntry {
                    engagement_id: Some(&input.engagement_id),
                    actor_id: Some(&actor),
                    action: "evidence.added",
                    entity_type: "evidence",
                    entity_id: Some(&evidence_id),
                    metadata: serde_json::json!({
                        "filename": evidence.filename,
                        "size": evidence.size,
                        "findingId": evidence.finding_id,
                        "testCaseId": evidence.test_case_id,
                    }),
                    created_at: &now,
                },
            )?;
            tx.commit()?;
            Ok(evidence)
        })();

        if result.is_err() {
            let _ = fs::remove_file(&blob_path);
        }
        result
    })
}

#[tauri::command]
pub async fn get_evidence_data(state: State<'_, AppState>, id: String) -> AppResult<EvidenceData> {
    let data_dir = state.data_dir.clone();
    state.with_session(|session| {
        let (evidence, plaintext) = read_evidence_content(&data_dir, session, &id)?;
        Ok(EvidenceData {
            evidence,
            data_base64: BASE64.encode(plaintext.as_slice()),
        })
    })
}

pub(crate) fn read_evidence_content(
    data_dir: &Path,
    session: &UnlockedSession,
    id: &str,
) -> AppResult<(Evidence, Zeroizing<Vec<u8>>)> {
    let evidence = db::evidence::get(&session.conn, id)?;
    let (nonce_hex, stored_path) = db::evidence::blob(&session.conn, &evidence.blob_id)?;
    let path = validate_blob_path(data_dir, &stored_path)?;
    let ciphertext = fs::read(&path)?;
    if ciphertext.len() > MAX_EVIDENCE_BYTES + 64 {
        return Err(AppError::Internal("stored evidence is too large".into()));
    }
    let nonce = hex::decode(nonce_hex).map_err(|_| AppError::Internal("invalid nonce".into()))?;
    let key = crypto::derive_evidence_key(&session.master_key, &evidence.engagement_id);
    let aad = format!(
        "testtracker-evidence-v1:{}:{}",
        evidence.engagement_id, evidence.id
    );
    let plaintext = crypto::decrypt(&key, &nonce, &ciphertext, aad.as_bytes())?;
    let actual_hash = blake3::hash(&plaintext).to_hex().to_string();
    if actual_hash != evidence.plaintext_hash {
        return Err(AppError::Crypto);
    }
    Ok((evidence, Zeroizing::new(plaintext.to_vec())))
}

#[tauri::command]
pub async fn delete_evidence(state: State<'_, AppState>, id: String) -> AppResult<DeleteResult> {
    let data_dir = state.data_dir.clone();
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let evidence = db::evidence::get(&session.conn, &id)?;
        let (_, stored_path) = db::evidence::blob(&session.conn, &evidence.blob_id)?;
        let path = validate_blob_path(&data_dir, &stored_path)?;
        let actor = session.profile.id.clone();
        let now = db::now();
        let tx = session.conn.transaction()?;
        db::evidence::delete(&tx, &id)?;
        db::evidence::delete_blob(&tx, &evidence.blob_id)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&evidence.engagement_id),
                actor_id: Some(&actor),
                action: "evidence.deleted",
                entity_type: "evidence",
                entity_id: Some(&id),
                metadata: serde_json::json!({ "filename": evidence.filename }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        let _ = fs::remove_file(path);
        Ok(DeleteResult { id })
    })
}

fn validate_input(input: &EvidenceInput) -> AppResult<()> {
    if input.engagement_id.trim().is_empty() {
        return Err(AppError::Validation("Engagement is required.".into()));
    }
    let filename = input.filename.trim();
    if filename.is_empty() || filename.chars().count() > 255 {
        return Err(AppError::Validation(
            "Evidence filename must be between 1 and 255 characters.".into(),
        ));
    }
    if filename.contains('/') || filename.contains('\\') {
        return Err(AppError::Validation(
            "Evidence filename must not contain path separators.".into(),
        ));
    }
    if input.mime_type.chars().count() > 200 || input.caption.chars().count() > 2_000 {
        return Err(AppError::Validation(
            "An evidence field is too long.".into(),
        ));
    }
    Ok(())
}

fn evidence_root(data_dir: &Path) -> PathBuf {
    data_dir.join("evidence")
}

fn validate_blob_path(data_dir: &Path, stored: &str) -> AppResult<PathBuf> {
    let path = PathBuf::from(stored);
    if !path.starts_with(evidence_root(data_dir)) {
        return Err(AppError::Internal("invalid evidence path".into()));
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
