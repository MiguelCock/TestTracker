use std::time::{Duration, Instant};

use tauri::State;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::ai::model::{
    AiPolicy, AiPreview, AiPromptInput, AiPromptPreview, AiProviderInput, AiProviderSummary,
    AiResponse,
};
use crate::ai::{providers, redaction};
use crate::db;
use crate::domain::models::DeleteResult;
use crate::domain::rbac;
use crate::error::{AppError, AppResult};
use crate::security::{audit, crypto};
use crate::state::AppState;

const CONSENT_TTL_SECONDS: u64 = 5 * 60;

fn provider_aad(id: &str) -> String {
    format!("testtracker-ai-provider-v1:{id}")
}

#[tauri::command]
pub async fn list_ai_providers(state: State<'_, AppState>) -> AppResult<Vec<AiProviderSummary>> {
    state.with_session(|session| {
        let stored = db::ai::list(&session.conn)?;
        Ok(stored.into_iter().map(summary).collect())
    })
}

#[tauri::command]
pub async fn create_ai_provider(
    state: State<'_, AppState>,
    input: AiProviderInput,
) -> AppResult<AiProviderSummary> {
    let (name, base_url, model) = validate_provider_input(&input)?;

    state.with_session(|session| {
        if !rbac::can_manage(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let id = Uuid::now_v7().to_string();
        let encrypted_api_key = match input.api_key.as_deref().map(str::trim) {
            Some(key) if !key.is_empty() => crypto::encrypt_secret(
                &session.master_key,
                provider_aad(&id).as_bytes(),
                key.as_bytes(),
            )?,
            _ => String::new(),
        };
        let now = db::now();
        let stored = db::ai::StoredAiProvider {
            id: id.clone(),
            name: name.to_string(),
            kind: input.kind,
            base_url: base_url.to_string(),
            model: model.to_string(),
            encrypted_api_key,
            created_by: session.profile.id.clone(),
            created_at: now.clone(),
        };
        let tx = session.conn.transaction()?;
        db::ai::insert(&tx, &stored)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&session.profile.id),
                action: "ai_provider.created",
                entity_type: "ai_provider",
                entity_id: Some(&id),
                metadata: serde_json::json!({
                    "kind": stored.kind.as_str(),
                    "model": stored.model,
                    "local": providers::is_local(stored.kind, &stored.base_url),
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(summary(stored))
    })
}

#[tauri::command]
pub async fn update_ai_provider(
    state: State<'_, AppState>,
    id: String,
    input: AiProviderInput,
) -> AppResult<AiProviderSummary> {
    let (name, base_url, model) = validate_provider_input(&input)?;
    state.with_session(|session| {
        if !rbac::can_manage(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let existing = db::ai::get(&session.conn, &id)?;
        let endpoint_changed = existing.kind != input.kind || existing.base_url != base_url;
        let encrypted_api_key = match input.api_key.as_deref().map(str::trim) {
            Some(key) if !key.is_empty() => crypto::encrypt_secret(
                &session.master_key,
                provider_aad(&id).as_bytes(),
                key.as_bytes(),
            )?,
            _ if endpoint_changed && !existing.encrypted_api_key.is_empty() => {
                return Err(AppError::Validation(
                    "Re-enter the API key when changing the provider URL or type.".into(),
                ));
            }
            _ => existing.encrypted_api_key,
        };
        let stored = db::ai::StoredAiProvider {
            id: id.clone(),
            name,
            kind: input.kind,
            base_url,
            model,
            encrypted_api_key,
            created_by: existing.created_by,
            created_at: existing.created_at,
        };
        let now = db::now();
        let tx = session.conn.transaction()?;
        db::ai::update(&tx, &stored)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&session.profile.id),
                action: "ai_provider.updated",
                entity_type: "ai_provider",
                entity_id: Some(&id),
                metadata: serde_json::json!({
                    "kind": stored.kind.as_str(),
                    "model": stored.model,
                    "local": providers::is_local(stored.kind, &stored.base_url),
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(summary(stored))
    })
}

#[tauri::command]
pub async fn list_ai_models(
    state: State<'_, AppState>,
    kind: crate::ai::model::AiProviderKind,
    base_url: String,
    api_key: Option<String>,
    provider_id: Option<String>,
) -> AppResult<Vec<String>> {
    let key = state.with_session(|session| {
        if !rbac::can_manage(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        providers::validate_base_url(&base_url)?;
        let key = match provider_id.as_deref() {
            Some(id) => {
                let provider = db::ai::get(&session.conn, id)?;
                let endpoint_changed =
                    provider.kind != kind || provider.base_url != base_url.trim();
                match api_key.as_deref().map(str::trim) {
                    Some(api_key) if !api_key.is_empty() => Zeroizing::new(api_key.to_string()),
                    _ if endpoint_changed || provider.encrypted_api_key.is_empty() => {
                        Zeroizing::new(String::new())
                    }
                    _ => {
                        let bytes = crypto::decrypt_secret(
                            &session.master_key,
                            provider_aad(id).as_bytes(),
                            &provider.encrypted_api_key,
                        )?;
                        Zeroizing::new(
                            String::from_utf8(bytes.to_vec()).map_err(|_| AppError::Crypto)?,
                        )
                    }
                }
            }
            None => Zeroizing::new(api_key.unwrap_or_default()),
        };
        Ok(key)
    })?;
    providers::list_models(kind, &base_url, key.as_str()).await
}

#[tauri::command]
pub async fn delete_ai_provider(state: State<'_, AppState>, id: String) -> AppResult<DeleteResult> {
    state.with_session(|session| {
        if !rbac::can_manage(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let provider = db::ai::get(&session.conn, &id)?;
        let now = db::now();
        let tx = session.conn.transaction()?;
        db::ai::delete(&tx, &id)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&session.profile.id),
                action: "ai_provider.deleted",
                entity_type: "ai_provider",
                entity_id: Some(&id),
                metadata: serde_json::json!({
                    "kind": provider.kind.as_str(),
                    "model": provider.model,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(DeleteResult { id })
    })
}

#[tauri::command]
pub async fn get_ai_policy(
    state: State<'_, AppState>,
    engagement_id: String,
) -> AppResult<AiPolicy> {
    state.with_session(|session| db::ai::get_policy(&session.conn, &engagement_id))
}

#[tauri::command]
pub async fn set_ai_policy(
    state: State<'_, AppState>,
    engagement_id: String,
    policy: AiPolicy,
) -> AppResult<AiPolicy> {
    state.with_session(|session| {
        if !rbac::can_manage(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        db::test_cases::ensure_engagement(&session.conn, &engagement_id)?;
        let now = db::now();
        let tx = session.conn.transaction()?;
        db::ai::set_policy(&tx, &engagement_id, policy, &session.profile.id, &now)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&engagement_id),
                actor_id: Some(&session.profile.id),
                action: "ai_policy.updated",
                entity_type: "ai_policy",
                entity_id: Some(&engagement_id),
                metadata: serde_json::json!({ "policy": policy.as_str() }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(policy)
    })
}

#[tauri::command]
pub async fn preview_ai_prompt(
    state: State<'_, AppState>,
    input: AiPromptInput,
) -> AppResult<AiPromptPreview> {
    let prompt = input.prompt.trim();
    if prompt.is_empty() || prompt.chars().count() > 50_000 {
        return Err(AppError::Validation(
            "Prompt must be between 1 and 50,000 characters.".into(),
        ));
    }
    let (preview, response) = state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        db::test_cases::ensure_engagement(&session.conn, &input.engagement_id)?;
        let provider = db::ai::get(&session.conn, &input.provider_id)?;
        let local = providers::is_local(provider.kind, &provider.base_url);
        let policy = db::ai::get_policy(&session.conn, &input.engagement_id)?;
        if !policy.allows(local) {
            let message = match policy {
                AiPolicy::Off => "AI is disabled for this engagement.",
                AiPolicy::LocalOnly => {
                    "This provider is not local. Set the engagement AI policy to approved cloud first."
                }
                AiPolicy::ApprovedCloud => "The engagement AI policy does not allow this provider.",
            };
            return Err(AppError::Validation(message.into()));
        }

        let redacted = redaction::redact(prompt);
        let prompt_hash = blake3::hash(redacted.text.as_bytes())
            .to_hex()
            .to_string();
        let api_key = if provider.encrypted_api_key.is_empty() {
            Zeroizing::new(String::new())
        } else {
            let bytes = crypto::decrypt_secret(
                &session.master_key,
                provider_aad(&provider.id).as_bytes(),
                &provider.encrypted_api_key,
            )?;
            Zeroizing::new(
                String::from_utf8(bytes.to_vec())
                    .map_err(|_| AppError::Crypto)?,
            )
        };
        let id = Uuid::now_v7().to_string();
        let now = db::now();
        let preview = AiPreview {
            id: id.clone(),
            engagement_id: input.engagement_id.clone(),
            provider_id: provider.id.clone(),
            provider_name: provider.name.clone(),
            kind: provider.kind,
            base_url: provider.base_url.clone(),
            model: provider.model.clone(),
            api_key,
            system_prompt: input.purpose.system_prompt().to_string(),
            prompt: redacted.text.clone(),
            purpose: input.purpose,
            prompt_hash: prompt_hash.clone(),
            redaction_count: redacted.count,
            expires_at: Instant::now() + Duration::from_secs(CONSENT_TTL_SECONDS),
        };
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&input.engagement_id),
                actor_id: Some(&session.profile.id),
                action: "ai.preview",
                entity_type: "ai_provider",
                entity_id: Some(&provider.id),
                metadata: serde_json::json!({
                    "provider": provider.name,
                    "model": provider.model,
                    "purpose": input.purpose.as_str(),
                    "redactionCount": redacted.count,
                    "promptHash": prompt_hash,
                    "local": local,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        let response = AiPromptPreview {
            consent_id: id,
            provider_name: provider.name,
            model: provider.model,
            policy,
            redacted_prompt: redacted.text,
            redaction_count: redacted.count,
            redactions: redacted.labels,
            expires_in_seconds: CONSENT_TTL_SECONDS,
            is_local: local,
        };
        Ok((preview, response))
    })?;
    state.insert_ai_preview(preview);
    Ok(response)
}

#[tauri::command]
pub async fn send_ai_message(
    state: State<'_, AppState>,
    consent_id: String,
) -> AppResult<AiResponse> {
    let preview = state.take_ai_preview(&consent_id).ok_or_else(|| {
        AppError::Validation(
            "The AI preview expired or was already used. Preview the prompt again.".into(),
        )
    })?;
    let local = providers::is_local(preview.kind, &preview.base_url);
    let policy_check = state.with_session(|session| {
        db::test_cases::ensure_engagement(&session.conn, &preview.engagement_id)?;
        db::ai::get(&session.conn, &preview.provider_id)?;
        let policy = db::ai::get_policy(&session.conn, &preview.engagement_id)?;
        if !policy.allows(local) {
            return Err(AppError::Validation(
                "The engagement AI policy no longer allows this provider.".into(),
            ));
        }
        Ok(())
    });
    if let Err(error) = policy_check {
        audit_failure(&state, &preview, &error);
        return Err(error);
    }
    let reply = match providers::send(&providers::ProviderRequest {
        kind: preview.kind,
        base_url: &preview.base_url,
        model: &preview.model,
        api_key: preview.api_key.as_str(),
        system_prompt: &preview.system_prompt,
        prompt: &preview.prompt,
    })
    .await
    {
        Ok(reply) => reply,
        Err(error) => {
            audit_failure(&state, &preview, &error);
            return Err(error);
        }
    };
    let response_hash = blake3::hash(reply.content.as_bytes()).to_hex().to_string();
    state.with_session(|session| {
        let now = db::now();
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&preview.engagement_id),
                actor_id: Some(&session.profile.id),
                action: "ai.message.sent",
                entity_type: "ai_provider",
                entity_id: Some(&preview.provider_id),
                metadata: serde_json::json!({
                    "provider": preview.provider_name,
                    "model": preview.model,
                    "purpose": preview.purpose.as_str(),
                    "promptHash": preview.prompt_hash,
                    "responseHash": response_hash,
                    "redactionCount": preview.redaction_count,
                    "promptTokens": reply.prompt_tokens,
                    "completionTokens": reply.completion_tokens,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(())
    })?;
    Ok(AiResponse {
        content: reply.content,
        provider_name: preview.provider_name,
        model: preview.model,
        prompt_hash: preview.prompt_hash,
        response_hash,
        redaction_count: preview.redaction_count,
        prompt_tokens: reply.prompt_tokens,
        completion_tokens: reply.completion_tokens,
    })
}

fn validate_provider_input(input: &AiProviderInput) -> AppResult<(String, String, String)> {
    let name = input.name.trim().to_string();
    let base_url = input.base_url.trim().to_string();
    let model = input.model.trim().to_string();
    if name.is_empty() || name.chars().count() > 120 {
        return Err(AppError::Validation(
            "Provider name must be between 1 and 120 characters.".into(),
        ));
    }
    if base_url.is_empty() || base_url.chars().count() > 500 {
        return Err(AppError::Validation(
            "Provider URL must be between 1 and 500 characters.".into(),
        ));
    }
    if model.is_empty() || model.chars().count() > 200 {
        return Err(AppError::Validation(
            "Model name must be between 1 and 200 characters.".into(),
        ));
    }
    providers::validate_base_url(&base_url)?;
    if let Some(api_key) = input.api_key.as_deref()
        && api_key.chars().count() > 4096
    {
        return Err(AppError::Validation("The API key is too long.".into()));
    }
    Ok((name, base_url, model))
}

fn audit_failure(state: &AppState, preview: &AiPreview, error: &AppError) {
    let reason = match error {
        AppError::External(message) => message.clone(),
        _ => error.user_message().to_string(),
    };
    let _ = state.with_session(|session| {
        let now = db::now();
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&preview.engagement_id),
                actor_id: Some(&session.profile.id),
                action: "ai.message.failed",
                entity_type: "ai_provider",
                entity_id: Some(&preview.provider_id),
                metadata: serde_json::json!({
                    "provider": preview.provider_name,
                    "model": preview.model,
                    "purpose": preview.purpose.as_str(),
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

fn summary(provider: db::ai::StoredAiProvider) -> AiProviderSummary {
    AiProviderSummary {
        is_local: providers::is_local(provider.kind, &provider.base_url),
        id: provider.id,
        name: provider.name,
        kind: provider.kind,
        base_url: provider.base_url,
        model: provider.model,
        has_key: !provider.encrypted_api_key.is_empty(),
        created_at: provider.created_at,
    }
}
