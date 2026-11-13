use std::time::{Duration, Instant};

use serde_json::Value;
use tauri::State;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::ai::agent::{AgentProposalDto, AgentProposalRecord, AgentSkill};
use crate::ai::assistant::{
    ASSISTANT_PREVIEW_TTL_SECONDS, AssistantAttachment, AssistantChatInput, AssistantFocusKind,
    AssistantPreviewDto, AssistantPreviewRecord, AssistantReplyDto, AssistantRunRecord,
    AssistantStepDto, ParsedAssistantStep, PendingToolCall, assistant_system_prompt,
    build_assistant_context, build_assistant_prompt, build_run_prompt, parse_assistant_step,
};
use crate::ai::image_prep::prepare_image;
use crate::ai::model::AiPolicy;
use crate::ai::providers::{self, ProviderImage};
use crate::ai::redaction;
use crate::commands::agent::inherit_finding_scores;
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
        let mut selected_files = Vec::new();
        for file_id in input.file_ids.iter().take(50) {
            if selected_files.contains(file_id) {
                continue;
            }
            let file = db::engagement_files::get(&session.conn, file_id)?;
            if file.file.engagement_id != input.engagement_id {
                return Err(AppError::Validation(
                    "A selected reference file does not belong to this engagement.".into(),
                ));
            }
            selected_files.push(file_id.clone());
            if let Some(files) = built.context.get_mut("referenceFiles").and_then(Value::as_array_mut) {
                files.push(serde_json::json!({
                    "id": file.file.id,
                    "kind": file.file.kind,
                    "filename": file.file.filename,
                    "summary": file.file.summary,
                }));
            } else {
                built.context["referenceFiles"] = serde_json::json!([{
                    "id": file.file.id,
                    "kind": file.file.kind,
                    "filename": file.file.filename,
                    "summary": file.file.summary,
                }]);
            }
        }
        let context_budget = (provider.context_window as usize)
            .saturating_mul(2)
            .clamp(8_000, 100_000);
        crate::ai::assistant::trim_assistant_context(&mut built.context, context_budget)?;
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
            context_window: provider.context_window,
            api_key,
            system_prompt: assistant_system_prompt().to_string(),
            prompt: redacted.text.clone(),
            redaction_count: redacted.count,
            focus: input.focus.clone(),
            target_label: built.target_label,
            attachment,
            allowed_files: selected_files,
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
) -> AppResult<AssistantStepDto> {
    let preview = state.take_assistant_preview(&consent_id).ok_or_else(|| {
        AppError::Validation(
            "The assistant preview expired or was already used. Preview the message again.".into(),
        )
    })?;
    check_preview_provider(&state, &preview)?;
    let run = AssistantRunRecord {
        id: Uuid::now_v7().to_string(),
        engagement_id: preview.engagement_id.clone(),
        provider_name: preview.provider_name.clone(),
        kind: preview.kind,
        base_url: preview.base_url.clone(),
        model: preview.model.clone(),
        api_key: preview.api_key.clone(),
        context_window: preview.context_window,
        system_prompt: preview.system_prompt.clone(),
        base_prompt: preview.prompt.clone(),
        focus: preview.focus.clone(),
        allowed_files: preview.allowed_files.clone(),
        attachment: preview.attachment,
        redaction_count: preview.redaction_count,
        tool_results: Vec::new(),
        rounds: 0,
        calls: 0,
        repairs: 0,
        pending: None,
        expires_at: Instant::now() + Duration::from_secs(15 * 60),
    };
    advance_assistant_run(&state, run).await
}

#[tauri::command]
pub async fn resolve_assistant_tool(
    state: State<'_, AppState>,
    run_id: String,
    call_id: String,
    decision: String,
) -> AppResult<AssistantStepDto> {
    let mut run = state.take_assistant_run(&run_id).ok_or_else(|| {
        AppError::Validation("The assistant run expired or was already completed.".into())
    })?;
    let pending = run.pending.take().ok_or_else(|| {
        AppError::Validation("The assistant is not waiting for a tool decision.".into())
    })?;
    if pending.id != call_id {
        state.insert_assistant_run(run);
        return Err(AppError::Validation(
            "The tool request no longer matches.".into(),
        ));
    }
    if decision == "deny" {
        audit_tool_decision(&state, &run, &pending, "denied");
        return Ok(AssistantStepDto::Cancelled {
            message: "Tool call denied. The assistant run was stopped.".into(),
        });
    }
    if decision != "allow_once" && decision != "always" {
        state.insert_assistant_run(run);
        return Err(AppError::Validation(
            "Invalid tool permission decision.".into(),
        ));
    }
    if decision == "always" {
        state.with_session(|session| {
            let now = db::now();
            let tx = session.conn.transaction()?;
            db::engagement_files::allow_tool(
                &tx,
                &run.engagement_id,
                &pending.tool,
                &session.profile.id,
                &now,
            )?;
            audit::append(
                &tx,
                audit::AuditEntry {
                    engagement_id: Some(&run.engagement_id),
                    actor_id: Some(&session.profile.id),
                    action: "tool_permission.allowed",
                    entity_type: "tool_permission",
                    entity_id: Some(&pending.tool),
                    metadata: serde_json::json!({ "tool": pending.tool, "scope": "engagement" }),
                    created_at: &now,
                },
            )?;
            tx.commit()?;
            Ok(())
        })?;
    }
    audit_tool_decision(
        &state,
        &run,
        &pending,
        if decision == "always" {
            "always_allowed"
        } else {
            "allowed"
        },
    );
    let result = execute_assistant_tool(&state, &run, &pending.tool, &pending.arguments)?;
    append_tool_result(&state, &mut run, &pending.tool, &pending.arguments, result)?;
    advance_assistant_run(&state, run).await
}

fn check_preview_provider(state: &AppState, preview: &AssistantPreviewRecord) -> AppResult<()> {
    let local = providers::is_local(preview.kind, &preview.base_url);
    state.with_session(|session| {
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
    })
}

fn provider_request<'a>(
    run: &'a AssistantRunRecord,
    prompt: &'a str,
    schema: &'a Value,
    image: Option<&'a ProviderImage<'a>>,
) -> providers::ProviderRequest<'a> {
    providers::ProviderRequest {
        kind: run.kind,
        base_url: &run.base_url,
        model: &run.model,
        api_key: run.api_key.as_str(),
        system_prompt: &run.system_prompt,
        prompt,
        image,
        json_mode: true,
        json_schema: Some(schema),
        context_window: Some(run.context_window),
    }
}

async fn advance_assistant_run(
    state: &AppState,
    mut run: AssistantRunRecord,
) -> AppResult<AssistantStepDto> {
    if run.rounds >= 6 || run.calls >= 12 {
        return Ok(AssistantStepDto::Cancelled {
            message: "The assistant reached its tool or provider limit.".into(),
        });
    }
    let mut prompt = build_run_prompt(&run.base_prompt, &run.tool_results);
    let schema = crate::ai::assistant::assistant_response_schema();
    let image = run.attachment.as_ref().map(|image| ProviderImage {
        mime_type: &image.mime_type,
        data_base64: image.data_base64.as_str(),
    });
    let mut reply =
        providers::send(&provider_request(&run, &prompt, &schema, image.as_ref())).await?;
    run.rounds += 1;
    let mut parsed = parse_assistant_step(&reply.content);
    if parsed.is_err() && run.repairs == 0 {
        run.repairs += 1;
        prompt = crate::ai::agent::build_repair_prompt(&prompt, &reply.content);
        reply = providers::send(&provider_request(&run, &prompt, &schema, image.as_ref())).await?;
        run.rounds += 1;
        parsed = parse_assistant_step(&reply.content);
    }
    let parsed = match parsed {
        Ok(parsed) => parsed,
        Err(_) => {
            return Ok(AssistantStepDto::Cancelled {
                message:
                    "The model did not return a usable JSON response after one repair attempt. Try the provider diagnostic or a stronger model."
                        .into(),
            });
        }
    };
    let prompt_hash = blake3::hash(prompt.as_bytes()).to_hex().to_string();
    let response_hash = blake3::hash(reply.content.as_bytes()).to_hex().to_string();
    match parsed {
        ParsedAssistantStep::Tool { tool, arguments } => {
            if !crate::ai::tools::is_known_tool(&tool) {
                return Err(AppError::External(
                    "The assistant requested an unknown tool.".into(),
                ));
            }
            let allowed = state.with_session(|session| {
                db::engagement_files::tool_allowed(&session.conn, &run.engagement_id, &tool)
            })?;
            if allowed {
                let result = execute_assistant_tool(state, &run, &tool, &arguments)?;
                append_tool_result(state, &mut run, &tool, &arguments, result)?;
                return Box::pin(advance_assistant_run(state, run)).await;
            }
            let call_id = Uuid::now_v7().to_string();
            let run_id = run.id.clone();
            run.pending = Some(PendingToolCall {
                id: call_id.clone(),
                tool: tool.clone(),
                arguments: arguments.clone(),
            });
            state.insert_assistant_run(run);
            Ok(AssistantStepDto::ToolRequest {
                run_id,
                call_id,
                tool,
                arguments,
            })
        }
        ParsedAssistantStep::Final(parsed) => {
            let mut proposal_dto = None;
            if let Some(action) = parsed.action {
                let proposal = state.with_session(|session| {
                    if !rbac::can_write(session.profile.role) {
                        return Err(AppError::PermissionDenied);
                    }
                    validate_action_target(
                        &session.conn,
                        &run.engagement_id,
                        action.skill,
                        action.target_id.as_deref(),
                    )?;
                    let payload = if action.skill == AgentSkill::FindingImprove {
                        inherit_finding_scores(
                            &session.conn,
                            &run.engagement_id,
                            action.target_id.as_deref(),
                            action.payload.clone(),
                        )?
                    } else {
                        action.payload.clone()
                    };
                    let now = db::now();
                    let proposal = AgentProposalRecord {
                        id: Uuid::now_v7().to_string(),
                        engagement_id: run.engagement_id.clone(),
                        skill: action.skill,
                        target_id: action.target_id.clone(),
                        payload: payload.clone(),
                        expires_at: Instant::now() + Duration::from_secs(PROPOSAL_TTL_SECONDS),
                    };
                    let tx = session.conn.transaction()?;
                    audit::append(
                        &tx,
                        audit::AuditEntry {
                            engagement_id: Some(&run.engagement_id),
                            actor_id: Some(&session.profile.id),
                            action: "assistant.proposal",
                            entity_type: "assistant",
                            entity_id: Some(&proposal.id),
                            metadata: serde_json::json!({
                                "skill": proposal.skill.as_str(),
                                "targetId": proposal.target_id,
                                "promptHash": prompt_hash,
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
                        engagement_id: Some(&run.engagement_id),
                        actor_id: Some(&session.profile.id),
                        action: "assistant.reply",
                        entity_type: "assistant",
                        entity_id: Some(&run.id),
                        metadata: serde_json::json!({
                            "provider": run.provider_name,
                            "model": run.model,
                            "focus": run.focus.kind_str(),
                            "promptHash": prompt_hash,
                            "responseHash": response_hash,
                            "rounds": run.rounds,
                            "toolCalls": run.calls,
                            "promptTokens": reply.prompt_tokens,
                            "completionTokens": reply.completion_tokens,
                            "action": proposal_dto.as_ref().map(|proposal| proposal.skill.as_str()),
                            "image": run.attachment.is_some(),
                        }),
                        created_at: &now,
                    },
                )?;
                tx.commit()?;
                Ok(())
            })?;
            Ok(AssistantStepDto::Final {
                reply: AssistantReplyDto {
                    message: parsed.message,
                    proposal: proposal_dto,
                    provider_name: run.provider_name,
                    model: run.model,
                    prompt_hash,
                    response_hash,
                    redaction_count: run.redaction_count,
                    prompt_tokens: reply.prompt_tokens,
                    completion_tokens: reply.completion_tokens,
                },
            })
        }
    }
}

fn execute_assistant_tool(
    state: &AppState,
    run: &AssistantRunRecord,
    tool: &str,
    arguments: &Value,
) -> AppResult<Value> {
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        crate::ai::tools::execute_tool(
            &session.conn,
            &run.engagement_id,
            &run.allowed_files,
            tool,
            arguments,
        )
    })
}

fn append_tool_result(
    state: &AppState,
    run: &mut AssistantRunRecord,
    tool: &str,
    arguments: &Value,
    result: Value,
) -> AppResult<()> {
    let result_text = serde_json::to_string(&result)?;
    let redacted = redaction::redact(&result_text);
    let clipped: String = redacted.text.chars().take(8_000).collect();
    let argument_hash = blake3::hash(&serde_json::to_vec(arguments)?)
        .to_hex()
        .to_string();
    let result_hash = blake3::hash(result_text.as_bytes()).to_hex().to_string();
    run.tool_results.push(serde_json::json!({
        "tool": tool,
        "result": clipped,
        "truncated": redacted.text.chars().count() > 8_000,
    }));
    run.calls += 1;
    state.with_session(|session| {
        let now = db::now();
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&run.engagement_id),
                actor_id: Some(&session.profile.id),
                action: "assistant.tool",
                entity_type: "assistant_tool",
                entity_id: Some(tool),
                metadata: serde_json::json!({
                    "tool": tool,
                    "argumentHash": argument_hash,
                    "resultHash": result_hash,
                    "redactionCount": redacted.count,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(())
    })
}

fn audit_tool_decision(
    state: &AppState,
    run: &AssistantRunRecord,
    pending: &PendingToolCall,
    decision: &str,
) {
    let argument_hash = serde_json::to_vec(&pending.arguments)
        .map(|bytes| blake3::hash(&bytes).to_hex().to_string())
        .unwrap_or_default();
    let _ = state.with_session(|session| {
        let now = db::now();
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&run.engagement_id),
                actor_id: Some(&session.profile.id),
                action: "assistant.tool_decision",
                entity_type: "assistant_tool",
                entity_id: Some(&pending.tool),
                metadata: serde_json::json!({
                    "tool": pending.tool,
                    "decision": decision,
                    "argumentHash": argument_hash,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(())
    });
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
