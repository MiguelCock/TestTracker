use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use tauri::State;

use crate::bundles::format;
use crate::bundles::model::{BundleExportResult, BundleImportResult};
use crate::bundles::service;
use crate::db;
use crate::domain::rbac;
use crate::error::{AppError, AppResult};
use crate::security::audit;
use crate::state::AppState;

#[tauri::command]
pub async fn export_bundle(
    state: State<'_, AppState>,
    engagement_id: String,
    passphrase: String,
    include_documents: bool,
) -> AppResult<BundleExportResult> {
    let data_dir = state.data_dir.clone();
    state.with_session(|session| {
        let now = db::now();
        let result = service::export_bundle(
            &session.conn,
            &session.master_key,
            &data_dir,
            &engagement_id,
            include_documents,
            &passphrase,
            &session.profile,
            &now,
        )?;
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&engagement_id),
                actor_id: Some(&session.profile.id),
                action: "bundle.exported",
                entity_type: "engagement",
                entity_id: Some(&engagement_id),
                metadata: serde_json::json!({
                    "size": result.size,
                    "evidenceCount": result.evidence_count,
                    "documentCount": result.document_count,
                    "includeDocuments": include_documents,
                    "fingerprint": result.signature_fingerprint,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(result)
    })
}

#[tauri::command]
pub async fn import_bundle(
    state: State<'_, AppState>,
    data_base64: String,
    passphrase: String,
) -> AppResult<BundleImportResult> {
    if data_base64.len() > (format::MAX_BUNDLE_BYTES / 3 + 1) * 4 {
        return Err(AppError::Validation("Bundle file is too large.".into()));
    }
    let bytes = BASE64
        .decode(data_base64.as_bytes())
        .map_err(|_| AppError::Validation("Bundle file is not valid base64.".into()))?;
    if bytes.len() > format::MAX_BUNDLE_BYTES {
        return Err(AppError::Validation("Bundle file is too large.".into()));
    }
    let data_dir = state.data_dir.clone();
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        service::import_bundle(
            &mut session.conn,
            &session.master_key,
            &data_dir,
            &bytes,
            &passphrase,
            &session.profile,
            &db::now(),
        )
    })
}
