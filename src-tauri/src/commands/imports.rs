use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use tauri::State;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::ai::{providers, redaction};
use crate::db;
use crate::domain::rbac;
use crate::error::{AppError, AppResult};
use crate::imports::extract::{extract_document, parse_tracker_xlsx};
use crate::imports::model::{
    ImportApplyResult, ImportDraftDto, ImportDraftRecord, ImportExtraction, ImportExtractionRecord,
    ImportPreviewDto, ImportPreviewRecord, validate_draft,
};
use crate::imports::service::{
    IMPORT_SYSTEM_PROMPT, apply_draft, build_import_prompt, parse_import_draft,
};
use crate::security::{audit, crypto};
use crate::state::AppState;

const EXTRACTION_TTL_SECONDS: u64 = 30 * 60;
const PREVIEW_TTL_SECONDS: u64 = 5 * 60;
const DRAFT_TTL_SECONDS: u64 = 30 * 60;

#[tauri::command]
pub async fn extract_import_document(
    state: State<'_, AppState>,
    filename: String,
    data_base64: String,
) -> AppResult<ImportExtraction> {
    let filename = filename.trim().to_string();
    if filename.is_empty() || filename.len() > 255 {
        return Err(AppError::Validation(
            "A valid file name is required.".into(),
        ));
    }
    let bytes = BASE64
        .decode(data_base64.as_bytes())
        .map_err(|_| AppError::Validation("The import file is not valid base64.".into()))?;
    let extracted = extract_document(&bytes, &filename)?;
    let draft = if extracted.format == "xlsx" {
        parse_tracker_xlsx(&bytes, &filename)?
    } else {
        None
    };
    if let Some(draft) = &draft {
        validate_draft(draft)?;
    }
    let extraction_id = Uuid::now_v7().to_string();
    let draft_id = draft.as_ref().map(|_| Uuid::now_v7().to_string());
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let now = db::now();
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&session.profile.id),
                action: "import.extracted",
                entity_type: "import",
                entity_id: Some(&extraction_id),
                metadata: serde_json::json!({
                    "filename": filename,
                    "format": extracted.format,
                    "textChars": extracted.text.chars().count(),
                    "truncated": extracted.truncated,
                    "deterministicDraft": draft.is_some(),
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(())
    })?;
    let text_chars = extracted.text.chars().count();
    let text_preview: String = extracted.text.chars().take(4_000).collect();
    state.insert_import_extraction(ImportExtractionRecord {
        id: extraction_id.clone(),
        filename: filename.clone(),
        format: extracted.format.clone(),
        text: extracted.text,
        expires_at: Instant::now() + Duration::from_secs(EXTRACTION_TTL_SECONDS),
    });
    if let (Some(draft_id), Some(draft)) = (draft_id.clone(), draft.clone()) {
        state.insert_import_draft(ImportDraftRecord {
            id: draft_id,
            draft,
            expires_at: Instant::now() + Duration::from_secs(DRAFT_TTL_SECONDS),
        });
    }
    Ok(ImportExtraction {
        extraction_id,
        filename,
        format: extracted.format,
        text_preview,
        text_chars,
        truncated: extracted.truncated,
        draft_id,
        draft,
    })
}

#[tauri::command]
pub async fn preview_import_ai(
    state: State<'_, AppState>,
    extraction_id: String,
    provider_id: String,
    allow_cloud: bool,
    instructions: String,
) -> AppResult<ImportPreviewDto> {
    let extraction = state.get_import_extraction(&extraction_id).ok_or_else(|| {
        AppError::Validation("The extracted document expired. Extract the file again.".into())
    })?;
    let prompt = build_import_prompt(&extraction.filename, &extraction.text, &instructions)?;
    let (preview, response) = state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let provider = db::ai::get(&session.conn, &provider_id)?;
        let local = providers::is_local(provider.kind, &provider.base_url);
        if !local && !allow_cloud {
            return Err(AppError::Validation(
                "This provider is not local. Confirm that the document may be sent to the cloud provider.".into(),
            ));
        }
        let redacted = redaction::redact(&prompt);
        let prompt_hash = blake3::hash(redacted.text.as_bytes())
            .to_hex()
            .to_string();
        let api_key = if provider.encrypted_api_key.is_empty() {
            Zeroizing::new(String::new())
        } else {
            let bytes = crypto::decrypt_secret(
                &session.master_key,
                format!("testtracker-ai-provider-v1:{}", provider.id).as_bytes(),
                &provider.encrypted_api_key,
            )?;
            Zeroizing::new(String::from_utf8(bytes.to_vec()).map_err(|_| AppError::Crypto)?)
        };
        let id = Uuid::now_v7().to_string();
        let now = db::now();
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&session.profile.id),
                action: "import.preview",
                entity_type: "import",
                entity_id: Some(&id),
                metadata: serde_json::json!({
                    "filename": extraction.filename,
                    "sourceFormat": extraction.format,
                    "provider": provider.name,
                    "model": provider.model,
                    "local": local,
                    "allowCloud": allow_cloud,
                    "redactionCount": redacted.count,
                    "promptHash": prompt_hash,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        let preview = ImportPreviewRecord {
            id: id.clone(),
            provider_id: provider.id.clone(),
            provider_name: provider.name.clone(),
            source_name: extraction.filename.clone(),
            kind: provider.kind,
            base_url: provider.base_url.clone(),
            model: provider.model.clone(),
            api_key,
            prompt: redacted.text.clone(),
            prompt_hash,
            expires_at: Instant::now() + Duration::from_secs(PREVIEW_TTL_SECONDS),
        };
        let response = ImportPreviewDto {
            consent_id: id,
            provider_name: provider.name,
            model: provider.model,
            redacted_prompt: redacted.text,
            redaction_count: redacted.count,
            redactions: redacted.labels,
            expires_in_seconds: PREVIEW_TTL_SECONDS,
            is_local: local,
        };
        Ok((preview, response))
    })?;
    state.insert_import_preview(preview);
    Ok(response)
}

#[tauri::command]
pub async fn run_import_ai(
    state: State<'_, AppState>,
    consent_id: String,
) -> AppResult<ImportDraftDto> {
    let preview = state.take_import_preview(&consent_id).ok_or_else(|| {
        AppError::Validation(
            "The import preview expired or was already used. Preview the document again.".into(),
        )
    })?;
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        Ok(())
    })?;
    let reply = match providers::send(&providers::ProviderRequest {
        kind: preview.kind,
        base_url: &preview.base_url,
        model: &preview.model,
        api_key: preview.api_key.as_str(),
        system_prompt: IMPORT_SYSTEM_PROMPT,
        prompt: &preview.prompt,
    })
    .await
    {
        Ok(reply) => reply,
        Err(error) => {
            audit_import_failure(&state, &preview, &error);
            return Err(error);
        }
    };
    let mut draft = match parse_import_draft(&reply.content) {
        Ok(draft) => draft,
        Err(error) => {
            audit_import_failure(&state, &preview, &error);
            return Err(error);
        }
    };
    draft.source_name = preview.source_name.clone();
    draft.source_format = "ai".into();
    let draft_id = Uuid::now_v7().to_string();
    let response_hash = blake3::hash(reply.content.as_bytes()).to_hex().to_string();
    state.with_session(|session| {
        let now = db::now();
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&session.profile.id),
                action: "import.proposal",
                entity_type: "import",
                entity_id: Some(&draft_id),
                metadata: serde_json::json!({
                    "providerId": preview.provider_id,
                    "provider": preview.provider_name,
                    "model": preview.model,
                    "promptHash": preview.prompt_hash,
                    "responseHash": response_hash,
                    "promptTokens": reply.prompt_tokens,
                    "completionTokens": reply.completion_tokens,
                    "testCaseCount": draft.test_cases.len(),
                    "findingCount": draft.findings.len(),
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(())
    })?;
    let dto = ImportDraftDto {
        draft_id: draft_id.clone(),
        draft: draft.clone(),
    };
    state.insert_import_draft(ImportDraftRecord {
        id: draft_id,
        draft,
        expires_at: Instant::now() + Duration::from_secs(DRAFT_TTL_SECONDS),
    });
    Ok(dto)
}

#[tauri::command]
pub async fn apply_import_draft(
    state: State<'_, AppState>,
    draft_id: String,
) -> AppResult<ImportApplyResult> {
    let record = state.take_import_draft(&draft_id).ok_or_else(|| {
        AppError::Validation(
            "The import draft expired or was already applied. Extract the document again.".into(),
        )
    })?;
    if let Err(error) = validate_draft(&record.draft) {
        state.insert_import_draft(record);
        return Err(error);
    }
    let backup = record.clone();
    let result = state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let now = db::now();
        let tx = session.conn.transaction()?;
        let result = apply_draft(
            &tx,
            &record.draft,
            &session.profile.id,
            &session.profile.display_name,
            &now,
        )?;
        tx.commit()?;
        Ok(result)
    });
    match result {
        Ok(result) => Ok(result),
        Err(error) => {
            if !matches!(
                error,
                AppError::Locked
                    | AppError::SessionExpired
                    | AppError::NotInitialized
                    | AppError::PermissionDenied
            ) {
                state.insert_import_draft(backup);
            }
            Err(error)
        }
    }
}

fn audit_import_failure(state: &AppState, preview: &ImportPreviewRecord, error: &AppError) {
    let reason = match error {
        AppError::External(message) | AppError::Plugin(message) => message.clone(),
        _ => error.user_message().to_string(),
    };
    let _ = state.with_session(|session| {
        let now = db::now();
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&session.profile.id),
                action: "import.failed",
                entity_type: "import",
                entity_id: Some(&preview.id),
                metadata: serde_json::json!({
                    "providerId": preview.provider_id,
                    "provider": preview.provider_name,
                    "model": preview.model,
                    "promptHash": preview.prompt_hash,
                    "reason": reason,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(())
    });
}
