use std::time::{Duration, Instant};

use tauri::State;
use uuid::Uuid;
use zeroize::Zeroizing;

use serde_json::Value;

use crate::ai::agent::{
    AgentApplyResult, AgentPreviewDto, AgentPreviewRecord, AgentProposalDto, AgentProposalRecord,
    AgentSkill, build_context, build_prompt, build_repair_prompt, parse_proposal,
};
use crate::ai::model::AiPolicy;
use crate::ai::{providers, redaction};
use crate::db;
use crate::domain::enums::{FindingStatus, RetestStatus, TestCaseStatus};
use crate::domain::models::{
    ContactInput, EngagementDetail, EngagementInput, FindingInput, RiskInput, ScopeAssetInput,
    TestCaseInput,
};
use crate::domain::rbac;
use crate::domain::severity::severity_for;
use crate::error::{AppError, AppResult};
use crate::security::{audit, crypto};
use crate::state::AppState;

const PREVIEW_TTL_SECONDS: u64 = 5 * 60;
const PROPOSAL_TTL_SECONDS: u64 = 15 * 60;

#[tauri::command]
pub async fn preview_agent_skill(
    state: State<'_, AppState>,
    engagement_id: String,
    provider_id: String,
    skill: AgentSkill,
    target_id: Option<String>,
    instructions: String,
) -> AppResult<AgentPreviewDto> {
    let (preview, response) = state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        db::test_cases::ensure_engagement(&session.conn, &engagement_id)?;
        let provider = db::ai::get(&session.conn, &provider_id)?;
        let local = providers::is_local(provider.kind, &provider.base_url);
        let policy = db::ai::get_policy(&session.conn, &engagement_id)?;
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
        let built = build_context(
            &session.conn,
            &engagement_id,
            skill,
            target_id.as_deref(),
        )?;
        let prompt = build_prompt(skill, &built.context, &instructions)?;
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
            Zeroizing::new(
                String::from_utf8(bytes.to_vec()).map_err(|_| AppError::Crypto)?,
            )
        };
        let id = Uuid::now_v7().to_string();
        let now = db::now();
        let preview = AgentPreviewRecord {
            id: id.clone(),
            engagement_id: engagement_id.clone(),
            provider_id: provider.id.clone(),
            kind: provider.kind,
            base_url: provider.base_url.clone(),
            model: provider.model.clone(),
            api_key,
            context_window: provider.context_window,
            skill,
            target_id: target_id.clone(),
            system_prompt: skill.system_prompt().to_string(),
            prompt: redacted.text.clone(),
            prompt_hash: prompt_hash.clone(),
            expires_at: Instant::now() + Duration::from_secs(PREVIEW_TTL_SECONDS),
        };
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&engagement_id),
                actor_id: Some(&session.profile.id),
                action: "agent.preview",
                entity_type: "agent",
                entity_id: Some(&id),
                metadata: serde_json::json!({
                    "skill": skill.as_str(),
                    "targetId": target_id,
                    "provider": provider.name,
                    "model": provider.model,
                    "redactionCount": redacted.count,
                    "promptHash": prompt_hash,
                    "local": local,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        let response = AgentPreviewDto {
            consent_id: id,
            skill,
            target_id: target_id.clone(),
            target_label: built.target_label,
            provider_name: provider.name,
            model: provider.model,
            policy,
            redacted_prompt: redacted.text,
            redaction_count: redacted.count,
            redactions: redacted.labels,
            expires_in_seconds: PREVIEW_TTL_SECONDS,
            is_local: local,
        };
        Ok((preview, response))
    })?;
    state.insert_agent_preview(preview);
    Ok(response)
}

#[tauri::command]
pub async fn run_agent_skill(
    state: State<'_, AppState>,
    consent_id: String,
) -> AppResult<AgentProposalDto> {
    let preview = state.take_agent_preview(&consent_id).ok_or_else(|| {
        AppError::Validation(
            "The agent preview expired or was already used. Preview the context again.".into(),
        )
    })?;
    let local = providers::is_local(preview.kind, &preview.base_url);
    let check = state.with_session(|session| {
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
    if let Err(error) = check {
        audit_agent_failure(&state, &preview, &error);
        return Err(error);
    }
    let schema = preview.skill.response_schema();
    let request = providers::ProviderRequest {
        kind: preview.kind,
        base_url: &preview.base_url,
        model: &preview.model,
        api_key: preview.api_key.as_str(),
        system_prompt: &preview.system_prompt,
        prompt: &preview.prompt,
        image: None,
        json_mode: true,
        json_schema: Some(&schema),
        context_window: Some(preview.context_window),
    };
    let mut reply = match providers::send(&request).await {
        Ok(reply) => reply,
        Err(error) => {
            audit_agent_failure(&state, &preview, &error);
            return Err(error);
        }
    };
    let mut repairs = 0usize;
    let mut parsed = parse_proposal(preview.skill, &reply.content);
    if parsed.is_err() {
        repairs += 1;
        let repair_prompt = build_repair_prompt(&preview.prompt, &reply.content);
        let repair_request = providers::ProviderRequest {
            prompt: &repair_prompt,
            ..request
        };
        reply = match providers::send(&repair_request).await {
            Ok(reply) => reply,
            Err(error) => {
                audit_agent_failure(&state, &preview, &error);
                return Err(error);
            }
        };
        parsed = parse_proposal(preview.skill, &reply.content);
    }
    let payload = match parsed {
        Ok(payload) => payload,
        Err(error) => {
            audit_agent_failure(&state, &preview, &error);
            return Err(error);
        }
    };
    let response_hash = blake3::hash(reply.content.as_bytes()).to_hex().to_string();
    let (proposal, payload) = state.with_session(|session| {
        let payload = if preview.skill == AgentSkill::FindingImprove {
            inherit_finding_scores(
                &session.conn,
                &preview.engagement_id,
                preview.target_id.as_deref(),
                payload,
            )?
        } else {
            payload
        };
        let proposal = AgentProposalRecord {
            id: Uuid::now_v7().to_string(),
            engagement_id: preview.engagement_id.clone(),
            skill: preview.skill,
            target_id: preview.target_id.clone(),
            payload: payload.clone(),
            expires_at: Instant::now() + Duration::from_secs(PROPOSAL_TTL_SECONDS),
        };
        let now = db::now();
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&preview.engagement_id),
                actor_id: Some(&session.profile.id),
                action: "agent.proposal",
                entity_type: "agent",
                entity_id: Some(&proposal.id),
                metadata: serde_json::json!({
                    "skill": preview.skill.as_str(),
                    "targetId": preview.target_id,
                    "promptHash": preview.prompt_hash,
                    "responseHash": response_hash,
                    "promptTokens": reply.prompt_tokens,
                    "completionTokens": reply.completion_tokens,
                    "repairs": repairs,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok((proposal, payload))
    })?;
    let dto = AgentProposalDto {
        proposal_id: proposal.id.clone(),
        skill: proposal.skill,
        target_id: proposal.target_id.clone(),
        payload,
        expires_in_seconds: PROPOSAL_TTL_SECONDS,
    };
    state.insert_agent_proposal(proposal);
    Ok(dto)
}

/// Carries the target finding's probability and impact into a finding-improvement
/// proposal so a weak model cannot silently reset the severity.
pub(crate) fn inherit_finding_scores(
    conn: &rusqlite::Connection,
    engagement_id: &str,
    target_id: Option<&str>,
    payload: Value,
) -> AppResult<Value> {
    let target_id = target_id
        .ok_or_else(|| AppError::Validation("The action is missing its target.".into()))?;
    let finding = db::findings::get(conn, target_id)?;
    if finding.engagement_id != engagement_id {
        return Err(AppError::Validation(
            "The target finding does not belong to this engagement.".into(),
        ));
    }
    let mut payload = payload;
    let object = payload.as_object_mut().ok_or_else(|| {
        AppError::External("The agent returned an invalid finding payload.".into())
    })?;
    object.insert("probability".into(), serde_json::json!(finding.probability));
    object.insert(
        "impactScore".into(),
        serde_json::json!(finding.impact_score),
    );
    Ok(payload)
}

#[tauri::command]
pub async fn apply_agent_proposal(
    state: State<'_, AppState>,
    proposal_id: String,
) -> AppResult<AgentApplyResult> {
    let proposal = state.take_agent_proposal(&proposal_id).ok_or_else(|| {
        AppError::Validation("The agent proposal expired or was already applied.".into())
    })?;
    let backup = proposal.clone();
    let result = state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let now = db::now();
        let tx = session.conn.transaction()?;
        let applied = apply_proposal(&tx, &proposal, &session.profile.display_name, &now)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&proposal.engagement_id),
                actor_id: Some(&session.profile.id),
                action: "agent.applied",
                entity_type: "agent",
                entity_id: Some(&proposal.id),
                metadata: serde_json::json!({
                    "skill": proposal.skill.as_str(),
                    "targetId": proposal.target_id,
                    "createdTestCases": applied.1,
                    "createdFindings": applied.2,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(applied)
    });
    match result {
        Ok((message, created_test_cases, created_findings)) => Ok(AgentApplyResult {
            message,
            created_test_cases,
            created_findings,
            updated_engagement: matches!(
                proposal.skill,
                AgentSkill::ApplicationSummary
                    | AgentSkill::AttackSurface
                    | AgentSkill::Assumptions
            ),
        }),
        Err(error) => {
            state.insert_agent_proposal(backup);
            Err(error)
        }
    }
}

pub(crate) fn apply_proposal(
    tx: &rusqlite::Transaction<'_>,
    proposal: &AgentProposalRecord,
    assignee: &str,
    now: &str,
) -> AppResult<(String, usize, usize)> {
    match proposal.skill {
        AgentSkill::ApplicationSummary | AgentSkill::AttackSurface | AgentSkill::Assumptions => {
            let detail = db::engagements::detail(tx, &proposal.engagement_id)?;
            let mut input = input_from_detail(&detail);
            match proposal.skill {
                AgentSkill::ApplicationSummary => {
                    input.application_summary = payload_string(&proposal.payload, "value")?;
                }
                AgentSkill::AttackSurface => {
                    input.attack_surface = payload_string(&proposal.payload, "value")?;
                }
                AgentSkill::Assumptions => {
                    input.assumptions = payload_strings(&proposal.payload, "values")?;
                }
                _ => unreachable!(),
            }
            db::engagements::update(tx, &proposal.engagement_id, &input, now)?;
            Ok((format!("Applied {}.", proposal.skill.label()), 0, 0))
        }
        AgentSkill::TestCases => {
            let existing = db::test_cases::list(tx, &proposal.engagement_id)?;
            let base_order = existing
                .iter()
                .map(|test_case| test_case.sort_order)
                .max()
                .unwrap_or(-1)
                + 1;
            let items = proposal
                .payload
                .get("testCases")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| AppError::External("Invalid agent proposal.".into()))?;
            let mut created = 0usize;
            for (index, item) in items.iter().enumerate() {
                let input = TestCaseInput {
                    engagement_id: proposal.engagement_id.clone(),
                    template_id: None,
                    title: payload_string(item, "title")?,
                    approach: payload_string(item, "approach").unwrap_or_default(),
                    category: payload_string(item, "category").unwrap_or_default(),
                    status: TestCaseStatus::NotStarted,
                    result: String::new(),
                    notes: payload_string(item, "expectedResult").unwrap_or_default(),
                    assignee: assignee.to_string(),
                    sort_order: base_order + index as i64,
                };
                db::test_cases::create(tx, &input, now)?;
                created += 1;
            }
            Ok((format!("Added {created} test case(s)."), created, 0))
        }
        AgentSkill::TestApproach => {
            let target_id = proposal
                .target_id
                .as_deref()
                .ok_or_else(|| AppError::Validation("Missing target test case.".into()))?;
            let test_case = db::test_cases::get(tx, target_id)?;
            if test_case.engagement_id != proposal.engagement_id {
                return Err(AppError::Validation(
                    "The target test case does not belong to this engagement.".into(),
                ));
            }
            let input = TestCaseInput {
                engagement_id: test_case.engagement_id,
                template_id: test_case.template_id,
                title: test_case.title,
                approach: payload_string(&proposal.payload, "approach")?,
                category: test_case.category,
                status: test_case.status,
                result: test_case.result,
                notes: test_case.notes,
                assignee: assignee.to_string(),
                sort_order: test_case.sort_order,
            };
            db::test_cases::update(tx, target_id, &input, now)?;
            Ok(("Updated the test approach.".into(), 0, 0))
        }
        AgentSkill::FindingImprove => {
            let target_id = proposal
                .target_id
                .as_deref()
                .ok_or_else(|| AppError::Validation("Missing target finding.".into()))?;
            let finding = db::findings::get(tx, target_id)?;
            if finding.engagement_id != proposal.engagement_id {
                return Err(AppError::Validation(
                    "The target finding does not belong to this engagement.".into(),
                ));
            }
            let probability = finding.probability;
            let impact_score = finding.impact_score;
            let severity = severity_for(probability, impact_score)?;
            let input = FindingInput {
                engagement_id: proposal.engagement_id.clone(),
                code: finding.code.clone(),
                title: payload_string(&proposal.payload, "title")?,
                description: payload_string(&proposal.payload, "description").unwrap_or_default(),
                impact: payload_string(&proposal.payload, "impact").unwrap_or_default(),
                remediation: payload_string(&proposal.payload, "remediation").unwrap_or_default(),
                owasp: payload_string(&proposal.payload, "owasp").unwrap_or_default(),
                cwe: payload_string(&proposal.payload, "cwe").unwrap_or_default(),
                status: finding.status,
                retest_status: finding.retest_status,
                owner: finding.owner,
                probability,
                impact_score,
                test_case_ids: finding.test_case_ids,
            };
            db::findings::update(tx, target_id, &input, severity, now)?;
            Ok(("Updated the finding draft.".into(), 0, 0))
        }
        AgentSkill::FindingDraft => {
            let target_id = proposal
                .target_id
                .as_deref()
                .ok_or_else(|| AppError::Validation("Missing target test case.".into()))?;
            let target_test = db::test_cases::get(tx, target_id)?;
            if target_test.engagement_id != proposal.engagement_id {
                return Err(AppError::Validation(
                    "The target test case does not belong to this engagement.".into(),
                ));
            }
            let probability = payload_u8(&proposal.payload, "probability")?;
            let impact_score = payload_u8(&proposal.payload, "impactScore")?;
            let severity = severity_for(probability, impact_score)?;
            let code = db::findings::next_code(tx, &proposal.engagement_id)?;
            let input = FindingInput {
                engagement_id: proposal.engagement_id.clone(),
                code: code.clone(),
                title: payload_string(&proposal.payload, "title")?,
                description: payload_string(&proposal.payload, "description").unwrap_or_default(),
                impact: payload_string(&proposal.payload, "impact").unwrap_or_default(),
                remediation: payload_string(&proposal.payload, "remediation").unwrap_or_default(),
                owasp: payload_string(&proposal.payload, "owasp").unwrap_or_default(),
                cwe: payload_string(&proposal.payload, "cwe").unwrap_or_default(),
                status: FindingStatus::Open,
                retest_status: RetestStatus::NotRequired,
                owner: assignee.to_string(),
                probability,
                impact_score,
                test_case_ids: vec![target_id.to_string()],
            };
            db::findings::create(tx, &input, severity, &code, now)?;
            Ok(("Created a draft finding.".into(), 0, 1))
        }
    }
}

fn input_from_detail(detail: &EngagementDetail) -> EngagementInput {
    EngagementInput {
        name: detail.engagement.name.clone(),
        client_code: detail.engagement.client_code.clone(),
        status: detail.engagement.status,
        start_date: detail.engagement.start_date.clone(),
        end_date: detail.engagement.end_date.clone(),
        application_summary: detail.engagement.application_summary.clone(),
        attack_surface: detail.engagement.attack_surface.clone(),
        scope_assets: detail
            .scope_assets
            .iter()
            .map(|asset| ScopeAssetInput {
                kind: asset.kind.clone(),
                value: asset.value.clone(),
                in_scope: asset.in_scope,
                notes: asset.notes.clone(),
            })
            .collect(),
        risks: detail
            .risks
            .iter()
            .map(|risk| RiskInput {
                title: risk.title.clone(),
                description: risk.description.clone(),
                likelihood: risk.likelihood,
                impact: risk.impact,
                notes: risk.notes.clone(),
            })
            .collect(),
        assumptions: detail
            .assumptions
            .iter()
            .map(|assumption| assumption.text.clone())
            .collect(),
        contacts: detail
            .contacts
            .iter()
            .map(|contact| ContactInput {
                name: contact.name.clone(),
                role: contact.role.clone(),
                email: contact.email.clone(),
                phone: contact.phone.clone(),
            })
            .collect(),
    }
}

fn payload_string(payload: &serde_json::Value, key: &str) -> AppResult<String> {
    payload
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| AppError::External("Invalid agent proposal.".into()))
}

fn payload_strings(payload: &serde_json::Value, key: &str) -> AppResult<Vec<String>> {
    payload
        .get(key)
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .ok_or_else(|| AppError::External("Invalid agent proposal.".into()))
}

fn payload_u8(payload: &serde_json::Value, key: &str) -> AppResult<u8> {
    payload
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .ok_or_else(|| AppError::External("Invalid agent proposal.".into()))
}

fn audit_agent_failure(state: &AppState, preview: &AgentPreviewRecord, error: &AppError) {
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
                action: "agent.failed",
                entity_type: "agent",
                entity_id: Some(&preview.id),
                metadata: serde_json::json!({
                    "skill": preview.skill.as_str(),
                    "targetId": preview.target_id,
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
