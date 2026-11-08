use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use tauri::State;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::db;
use crate::documents::build::ReportSource;
use crate::documents::model::{DocumentData, DocumentFormat, DocumentKind, GeneratedDocument};
use crate::domain::models::DeleteResult;
use crate::domain::rbac;
use crate::error::{AppError, AppResult};
use crate::security::{audit, crypto};
use crate::state::AppState;

#[tauri::command]
pub async fn generate_document(
    state: State<'_, AppState>,
    engagement_id: String,
    kind: DocumentKind,
    format: DocumentFormat,
) -> AppResult<GeneratedDocument> {
    if format == DocumentFormat::Xlsx && !kind.supports_xlsx() {
        return Err(AppError::Validation(
            "XLSX export is only available for the test case tracker and findings register.".into(),
        ));
    }
    let data_dir = state.data_dir.clone();
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let detail = db::engagements::detail(&session.conn, &engagement_id)?;
        let test_cases = db::test_cases::list(&session.conn, &engagement_id)?;
        let findings = db::findings::list(&session.conn, &engagement_id)?;
        let evidence = db::evidence::list(&session.conn, &engagement_id)?;
        let generated_at = db::now();
        let source = ReportSource {
            engagement: &detail.engagement,
            scope_assets: &detail.scope_assets,
            risks: &detail.risks,
            assumptions: &detail.assumptions,
            contacts: &detail.contacts,
            test_cases: &test_cases,
            findings: &findings,
            evidence: &evidence,
            generated_by: &session.profile.display_name,
            generated_at: &generated_at,
        };
        let rendered = Zeroizing::new(crate::documents::render(kind, format, &source)?);
        let content_hash = blake3::hash(&rendered).to_hex().to_string();
        let id = Uuid::now_v7().to_string();
        let title = format!("{} - {}", kind.label(), detail.engagement.name);
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
        let client = if detail.engagement.client_code.trim().is_empty() {
            "engagement"
        } else {
            &detail.engagement.client_code
        };
        let filename = format!(
            "{}_{}_{}_{}.{}",
            kind.as_str(),
            sanitize(client),
            timestamp,
            &id[..8],
            format.extension()
        );
        let key = crypto::derive_document_key(&session.master_key, &engagement_id);
        let aad = format!("testtracker-document-v1:{engagement_id}:{id}");
        let encrypted = crypto::encrypt(&key, &rendered, aad.as_bytes())?;
        let directory = documents_root(&data_dir).join(&engagement_id);
        fs::create_dir_all(&directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
        }
        let path = directory.join(format!("{id}.{}", format.extension()));
        write_private(&path, &encrypted.ciphertext)?;

        let document = GeneratedDocument {
            id: id.clone(),
            engagement_id: engagement_id.clone(),
            kind,
            format,
            title,
            filename,
            size: rendered.len() as i64,
            content_hash: content_hash.clone(),
            created_by: session.profile.id.clone(),
            created_at: generated_at.clone(),
        };
        let stored = db::generated_documents::StoredDocument {
            document: document.clone(),
            nonce: hex::encode(encrypted.nonce),
            path: path.to_string_lossy().to_string(),
        };
        let result = (|| -> AppResult<GeneratedDocument> {
            let tx = session.conn.transaction()?;
            db::generated_documents::insert(&tx, &stored)?;
            audit::append(
                &tx,
                audit::AuditEntry {
                    engagement_id: Some(&engagement_id),
                    actor_id: Some(&session.profile.id),
                    action: "document.generated",
                    entity_type: "document",
                    entity_id: Some(&id),
                    metadata: serde_json::json!({
                        "kind": kind.as_str(),
                        "format": format.as_str(),
                        "contentHash": content_hash,
                        "size": document.size,
                    }),
                    created_at: &generated_at,
                },
            )?;
            tx.commit()?;
            Ok(document)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&path);
        }
        result
    })
}

#[tauri::command]
pub async fn list_documents(
    state: State<'_, AppState>,
    engagement_id: String,
) -> AppResult<Vec<GeneratedDocument>> {
    state.with_session(|session| db::generated_documents::list(&session.conn, &engagement_id))
}

#[tauri::command]
pub async fn get_document_data(state: State<'_, AppState>, id: String) -> AppResult<DocumentData> {
    let data_dir = state.data_dir.clone();
    state.with_session(|session| {
        let stored = db::generated_documents::get(&session.conn, &id)?;
        let path = validate_document_path(&data_dir, &stored.path)?;
        let ciphertext = fs::read(&path)?;
        let nonce = hex::decode(&stored.nonce)
            .map_err(|_| AppError::Internal("invalid document nonce".into()))?;
        let key = crypto::derive_document_key(&session.master_key, &stored.document.engagement_id);
        let aad = format!(
            "testtracker-document-v1:{}:{}",
            stored.document.engagement_id, stored.document.id
        );
        let plaintext = crypto::decrypt(&key, &nonce, &ciphertext, aad.as_bytes())?;
        let actual_hash = blake3::hash(&plaintext).to_hex().to_string();
        if actual_hash != stored.document.content_hash {
            return Err(AppError::Crypto);
        }
        Ok(DocumentData {
            document: stored.document,
            data_base64: BASE64.encode(plaintext.as_slice()),
        })
    })
}

#[tauri::command]
pub async fn delete_document(state: State<'_, AppState>, id: String) -> AppResult<DeleteResult> {
    let data_dir = state.data_dir.clone();
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let stored = db::generated_documents::get(&session.conn, &id)?;
        let path = validate_document_path(&data_dir, &stored.path)?;
        let now = db::now();
        let tx = session.conn.transaction()?;
        db::generated_documents::delete(&tx, &id)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&stored.document.engagement_id),
                actor_id: Some(&session.profile.id),
                action: "document.deleted",
                entity_type: "document",
                entity_id: Some(&id),
                metadata: serde_json::json!({
                    "kind": stored.document.kind.as_str(),
                    "format": stored.document.format.as_str(),
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        let _ = fs::remove_file(path);
        Ok(DeleteResult { id })
    })
}

pub fn documents_root(data_dir: &Path) -> PathBuf {
    data_dir.join("documents")
}

fn validate_document_path(data_dir: &Path, stored: &str) -> AppResult<PathBuf> {
    let path = PathBuf::from(stored);
    if !path.starts_with(documents_root(data_dir)) {
        return Err(AppError::Internal("invalid document path".into()));
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

fn sanitize(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "engagement".into()
    } else {
        trimmed
    }
}
