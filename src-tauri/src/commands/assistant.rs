use std::time::{Duration, Instant};

use tauri::State;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::ai::agent::{AgentProposalDto, AgentProposalRecord, AgentSkill};
use crate::ai::assistant::{
    ASSISTANT_PREVIEW_TTL_SECONDS, AssistantAttachment, AssistantChatInput, AssistantFocusKind,
    AssistantPreviewDto, AssistantPreviewRecord, AssistantReplyDto, assistant_system_prompt,
    build_assistant_context, build_assistant_prompt, parse_assistant_reply,
};
use crate::ai::image_prep::prepare_image;
use crate::ai::model::AiPolicy;
use crate::ai::providers::{self, ProviderImage};
use crate::ai::redaction;
use crate::commands::evidence::read_evidence_content;
use crate::db;
use crate::domain::rbac;
use crate::error::{AppError, AppResult};
use crate::security::{audit, crypto};
use crate::state::AppState;

const PROPOSAL_TTL_SECONDS: u64 = 15 * 60;
const MAX_EVIDENCE_TEXT_CHARS: usize = 30_000;

#[tauri::command]
pub async fn preview_assistant_chat(
    state: State<'_, AppState>,
    input: AssistantChatInput,
) -> AppResult<AssistantPreviewDto> {
    input.focus.validate()?;
    let data_dir = state.data_dir.clone();
    let (preview, response) = state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        db::test_cases::ensure_engagement(&session.conn, &input.engagement_id)?;
        let provider = db::ai::get(&session.conn, &input.provider_id)?;
        let local = providers::is_local(provider.kind, &provider.base_url);
        let policy = db::ai::get_policy(&session.conn, &input.engagement_id)?;
        if !policy.allows(local) {
            return Err(AppError::Validation(policy_message(policy)));
        }
        let mut built = build_assistant_context(
            &session.conn,
            &input.engagement_id,
            &input.focus,
        )?;
        let mut attachment = None;
        let mut attachment_meta = None;
        if input.focus.kind == AssistantFocusKind::Evidence {
            let id = input.focus.id.as_deref().ok_or_else(|| {
                AppError::Validation("The selected evidence is missing an ID.".into())
            })?;
            let evidence = db::evidence::get(&session.conn, id)?;
            if evidence.engagement_id != input.engagement_id {
                return Err(AppError::Validation(
                    "The selected evidence does not belong to this engagement.".into(),
                ));
            }
            let (evidence, bytes) = read_evidence_content(&data_dir, session, id)?;
            if is_image(&evidence.mime_type) {
                if !provider.supports_vision {
                    return Err(AppError::Validation(
                        "This provider is not marked as vision-capable. Enable image input in the provider settings."
                            .into(),
                    ));
                }
                let prepared = prepare_image(&bytes)?;
                attachment_meta = Some(AssistantAttachment {
                    filename: evidence.filename.clone(),
                    mime_type: prepared.mime_type.clone(),
                    size: prepared.size as i64,
                    width: prepared.width,
                    height: prepared.height,
                    hash: prepared.sent_hash.clone(),
                    original_hash: prepared.original_hash.clone(),
                });
                attachment = Some(prepared);
            } else {
                let extracted = crate::imports::extract::extract_document(
                    &bytes,
                    &evidence.filename,
                )
                .map_err(|_| {
                    AppError::Validation(
                        "This evidence type has no extractable text. Image analysis is available for PNG, JPEG, and WebP."
                            .into(),
                    )
                })?;
                let text: String = extracted
                    .text
                    .chars()
                    .take(MAX_EVIDENCE_TEXT_CHARS)
                    .collect();
                if let Some(focus) = built.context.get_mut("focus") {
                    focus["evidenceText"] = serde_json::Value::String(text);
                }
            }
        }
        let prompt = build_assistant_prompt(
            &built.context,
            &input.focus,
            &input.history,
            &input.message,
            attachment_meta.as_ref(),
        )?;
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
                engagement_id: Some(&input.engagement_id),
                actor_id: Some(&session.profile.id),
                action: "assistant.preview",
                entity_type: "assistant",
                entity_id: Some(&id),
                metadata: serde_json::json!({
                    "provider": provider.name,
                    "model": provider.model,
                    "focus": input.focus.kind_str(),
                    "focusId": input.focus.id,
                    "redactionCount": redacted.count,
                    "promptHash": prompt_hash,
                    "local": local,
                    "image": attachment_meta.as_ref().map(|item| serde_json::json!({
                        "mimeType": item.mime_type,
                        "size": item.size,
                        "width": item.width,
                        "height": item.height,
                        "hash": item.hash,
                    })),
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        let preview = AssistantPreviewRecord {
            id: id.clone(),
            engagement_id: input.engagement_id.clone(),
            provider_id: provider.id.clone(),
            provider_name: provider.name.clone(),
            kind: provider.kind,
            base_url: provider.base_url.clone(),
            model: provider.model.clone(),
            api_key,
            system_prompt: assistant_system_prompt().to_string(),
            prompt: redacted.text.clone(),
            prompt_hash,
            redaction_count: redacted.count,
            focus: input.focus.clone(),
            target_label: built.target_label,
            attachment,
            expires_at: Instant::now() + Duration::from_secs(ASSISTANT_PREVIEW_TTL_SECONDS),
        };
        let response = AssistantPreviewDto {
            consent_id: id,
            provider_name: provider.name,
            model: provider.model,
            policy,
            redacted_prompt: redacted.text,
            redaction_count: redacted.count,
            redactions: redacted.labels,
            expires_in_seconds: ASSISTANT_PREVIEW_TTL_SECONDS,
            is_local: local,
            focus: input.focus.clone(),
            target_label: preview.target_label.clone(),
            attachment: attachment_meta,
        };
        Ok((preview, response))
    })?;
    state.insert_assistant_preview(preview);
    Ok(response)
}

#[tauri::command]
pub async fn run_assistant_chat(
    state: State<'_, AppState>,
    consent_id: String,
) -> AppResult<AssistantReplyDto> {
    let preview = state.take_assistant_preview(&consent_id).ok_or_else(|| {
        AppError::Validation(
            "The assistant preview expired or was already used. Preview the message again.".into(),
        )
    })?;
    let local = providers::is_local(preview.kind, &preview.base_url);
    let check = state.with_session(|session| {
        db::test_cases::ensure_engagement(&session.conn, &preview.engagement_id)?;
        let provider = db::ai::get(&session.conn, &preview.provider_id)?;
        if provider.kind != preview.kind
            || provider.base_url != preview.base_url
            || provider.model != preview.model
        {
            return Err(AppError::Validation(
                "The AI provider changed after the preview. Preview the message again.".into(),
            ));
        }
        if preview.attachment.is_some() && !provider.supports_vision {
            return Err(AppError::Validation(
                "The provider no longer supports image input.".into(),
            ));
        }
        let policy = db::ai::get_policy(&session.conn, &preview.engagement_id)?;
        if !policy.allows(local) {
            return Err(AppError::Validation(policy_message(policy)));
        }
        Ok(())
    });
    if let Err(error) = check {
        audit_assistant_failure(&state, &preview, &error);
        return Err(error);
    }
    let image = preview.attachment.as_ref().map(|image| ProviderImage {
        mime_type: &image.mime_type,
        data_base64: image.data_base64.as_str(),
    });
    let reply = match providers::send(&providers::ProviderRequest {
        kind: preview.kind,
        base_url: &preview.base_url,
        model: &preview.model,
        api_key: preview.api_key.as_str(),
        system_prompt: &preview.system_prompt,
        prompt: &preview.prompt,
        image: image.as_ref(),
    })
    .await
    {
        Ok(reply) => reply,
        Err(error) => {
            audit_assistant_failure(&state, &preview, &error);
            return Err(error);
        }
    };
    let parsed = match parse_assistant_reply(&reply.content) {
        Ok(parsed) => parsed,
        Err(error) => {
            audit_assistant_failure(&state, &preview, &error);
            return Err(error);
        }
    };
    let response_hash = blake3::hash(reply.content.as_bytes()).to_hex().to_string();
    let mut proposal_dto = None;
    if let Some(action) = parsed.action {
        let proposal = state.with_session(|session| {
            if !rbac::can_write(session.profile.role) {
                return Err(AppError::PermissionDenied);
            }
            validate_action_target(
                &session.conn,
                &preview.engagement_id,
                action.skill,
                action.target_id.as_deref(),
            )?;
            let now = db::now();
            let proposal = AgentProposalRecord {
                id: Uuid::now_v7().to_string(),
                engagement_id: preview.engagement_id.clone(),
                skill: action.skill,
                target_id: action.target_id.clone(),
                payload: action.payload.clone(),
                expires_at: Instant::now() + Duration::from_secs(PROPOSAL_TTL_SECONDS),
            };
            let tx = session.conn.transaction()?;
            audit::append(
                &tx,
                audit::AuditEntry {
                    engagement_id: Some(&preview.engagement_id),
                    actor_id: Some(&session.profile.id),
                    action: "assistant.proposal",
                    entity_type: "assistant",
                    entity_id: Some(&proposal.id),
                    metadata: serde_json::json!({
                        "skill": proposal.skill.as_str(),
                        "targetId": proposal.target_id,
                        "promptHash": preview.prompt_hash,
                        "responseHash": response_hash,
                    }),
                    created_at: &now,
                },
            )?;
            tx.commit()?;
            Ok(proposal)
        })?;
        proposal_dto = Some(AgentProposalDto {
            proposal_id: proposal.id.clone(),
            skill: proposal.skill,
            target_id: proposal.target_id.clone(),
            payload: proposal.payload.clone(),
            expires_in_seconds: PROPOSAL_TTL_SECONDS,
        });
        state.insert_agent_proposal(proposal);
    }
    state.with_session(|session| {
        let now = db::now();
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&preview.engagement_id),
                actor_id: Some(&session.profile.id),
                action: "assistant.reply",
                entity_type: "assistant",
                entity_id: Some(&preview.id),
                metadata: serde_json::json!({
                    "provider": preview.provider_name,
                    "model": preview.model,
                    "focus": preview.focus.kind_str(),
                    "promptHash": preview.prompt_hash,
                    "responseHash": response_hash,
                    "promptTokens": reply.prompt_tokens,
                    "completionTokens": reply.completion_tokens,
                    "action": proposal_dto.as_ref().map(|proposal| proposal.skill.as_str()),
                    "image": preview.attachment.is_some(),
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(())
    })?;
    Ok(AssistantReplyDto {
        message: parsed.message,
        proposal: proposal_dto,
        provider_name: preview.provider_name,
        model: preview.model,
        prompt_hash: preview.prompt_hash,
        response_hash,
        redaction_count: preview.redaction_count,
        prompt_tokens: reply.prompt_tokens,
        completion_tokens: reply.completion_tokens,
    })
}

fn validate_action_target(
    conn: &rusqlite::Connection,
    engagement_id: &str,
    skill: AgentSkill,
    target_id: Option<&str>,
) -> AppResult<()> {
    match skill {
        AgentSkill::TestApproach | AgentSkill::FindingDraft => {
            let target = target_id
                .ok_or_else(|| AppError::Validation("The action is missing its target.".into()))?;
            let test = db::test_cases::get(conn, target)?;
            if test.engagement_id != engagement_id {
                return Err(AppError::Validation(
                    "The target test case does not belong to this engagement.".into(),
                ));
            }
        }
        AgentSkill::FindingImprove => {
            let target = target_id
                .ok_or_else(|| AppError::Validation("The action is missing its target.".into()))?;
            let finding = db::findings::get(conn, target)?;
            if finding.engagement_id != engagement_id {
                return Err(AppError::Validation(
                    "The target finding does not belong to this engagement.".into(),
                ));
            }
        }
        _ => {
            if target_id.is_some() {
                return Err(AppError::Validation(
                    "This action does not accept a target.".into(),
                ));
            }
        }
    }
    Ok(())
}

fn audit_assistant_failure(state: &AppState, preview: &AssistantPreviewRecord, error: &AppError) {
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
                engagement_id: Some(&preview.engagement_id),
                actor_id: Some(&session.profile.id),
                action: "assistant.failed",
                entity_type: "assistant",
                entity_id: Some(&preview.id),
                metadata: serde_json::json!({
                    "focus": preview.focus.kind_str(),
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

fn policy_message(policy: AiPolicy) -> String {
    match policy {
        AiPolicy::Off => "AI is disabled for this engagement.",
        AiPolicy::LocalOnly => {
            "This provider is not local. Set the engagement AI policy to approved cloud first."
        }
        AiPolicy::ApprovedCloud => "The engagement AI policy does not allow this provider.",
    }
    .to_string()
}

fn is_image(mime: &str) -> bool {
    matches!(
        mime,
        "image/png" | "image/jpeg" | "image/jpg" | "image/webp"
    )
}
