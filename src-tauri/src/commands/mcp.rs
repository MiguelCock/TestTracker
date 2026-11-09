use std::time::Instant;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use tauri::{AppHandle, State};
use zeroize::Zeroizing;

use crate::ai::agent::AgentProposalRecord;
use crate::commands::agent::apply_proposal;
use crate::db;
use crate::domain::rbac;
use crate::error::{AppError, AppResult};
use crate::mcp::model::{McpApprovalDto, McpApprovalResult, McpApprovalStatus, McpStatus};
use crate::mcp::runtime::DEFAULT_PORT;
use crate::security::{audit, crypto};
use crate::state::{AppState, UnlockedSession};

const TOKEN_KEY: &str = "mcp.token";
const PORT_KEY: &str = "mcp.port";

#[tauri::command]
pub async fn mcp_status(state: State<'_, AppState>) -> AppResult<McpStatus> {
    state.with_session_no_touch(|session| {
        let has_token = db::settings::get(&session.conn, TOKEN_KEY)?.is_some();
        let stored_port = db::settings::get(&session.conn, PORT_KEY)?
            .and_then(|value| value.parse::<u16>().ok())
            .filter(|value| (1024..=u16::MAX).contains(value))
            .unwrap_or(DEFAULT_PORT);
        let mut status = state.mcp.status(has_token);
        if !status.running {
            status.port = stored_port;
        }
        Ok(status)
    })
}

#[tauri::command]
pub async fn mcp_start(
    app: AppHandle,
    state: State<'_, AppState>,
    port: Option<u16>,
) -> AppResult<McpStatus> {
    if state.mcp.is_running() {
        return Err(AppError::Conflict(
            "The MCP bridge is already running.".into(),
        ));
    }
    let requested_port = port.map(validate_port).transpose()?.unwrap_or(DEFAULT_PORT);
    let (token, selected_port) = state.with_session(|session| {
        if !rbac::can_manage(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let stored_port = db::settings::get(&session.conn, PORT_KEY)?
            .and_then(|value| value.parse::<u16>().ok())
            .filter(|value| (1024..=u16::MAX).contains(value))
            .unwrap_or(DEFAULT_PORT);
        let selected_port = if port.is_some() {
            requested_port
        } else {
            stored_port
        };
        if port.is_some() && selected_port != stored_port {
            let now = db::now();
            let tx = session.conn.transaction()?;
            db::settings::set(&tx, PORT_KEY, &selected_port.to_string())?;
            audit::append(
                &tx,
                audit::AuditEntry {
                    engagement_id: None,
                    actor_id: Some(&session.profile.id),
                    action: "mcp.settings.updated",
                    entity_type: "mcp",
                    entity_id: None,
                    metadata: serde_json::json!({ "port": selected_port }),
                    created_at: &now,
                },
            )?;
            tx.commit()?;
        }
        let token = ensure_token(session)?;
        Ok((token, selected_port))
    })?;
    let actual_port = state.mcp.start(app, selected_port, token.as_str()).await?;
    let audit_result = state.with_session_no_touch(|session| {
        let now = db::now();
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&session.profile.id),
                action: "mcp.server.started",
                entity_type: "mcp",
                entity_id: None,
                metadata: serde_json::json!({
                    "port": actual_port,
                    "endpoint": format!("http://127.0.0.1:{actual_port}/mcp"),
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(())
    });
    if let Err(error) = audit_result {
        state.mcp.stop();
        return Err(error);
    }
    state.with_session_no_touch(|session| {
        let has_token = db::settings::get(&session.conn, TOKEN_KEY)?.is_some();
        Ok(state.mcp.status(has_token))
    })
}

#[tauri::command]
pub async fn mcp_stop(state: State<'_, AppState>) -> AppResult<McpStatus> {
    state.with_session(|session| {
        if !rbac::can_manage(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let now = db::now();
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&session.profile.id),
                action: "mcp.server.stopped",
                entity_type: "mcp",
                entity_id: None,
                metadata: serde_json::json!({}),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        state.mcp.stop();
        let has_token = db::settings::get(&session.conn, TOKEN_KEY)?.is_some();
        Ok(state.mcp.status(has_token))
    })
}

#[tauri::command]
pub async fn mcp_reveal_token(state: State<'_, AppState>) -> AppResult<String> {
    state.with_session(|session| {
        if !rbac::can_manage(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let token = load_token(session)?;
        let now = db::now();
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&session.profile.id),
                action: "mcp.token.revealed",
                entity_type: "mcp",
                entity_id: None,
                metadata: serde_json::json!({}),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(token.to_string())
    })
}

#[tauri::command]
pub async fn mcp_regenerate_token(state: State<'_, AppState>) -> AppResult<String> {
    if state.mcp.is_running() {
        return Err(AppError::Conflict(
            "Stop the MCP bridge before regenerating its token.".into(),
        ));
    }
    state.with_session(|session| {
        if !rbac::can_manage(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let token = Zeroizing::new(URL_SAFE_NO_PAD.encode(crypto::random_bytes::<32>()));
        let encrypted = crypto::encrypt_secret(
            &session.master_key,
            token_aad(&session.profile.id).as_bytes(),
            token.as_bytes(),
        )?;
        let now = db::now();
        let tx = session.conn.transaction()?;
        db::settings::set(&tx, TOKEN_KEY, &encrypted)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&session.profile.id),
                action: "mcp.token.regenerated",
                entity_type: "mcp",
                entity_id: None,
                metadata: serde_json::json!({}),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(token.to_string())
    })
}

#[tauri::command]
pub async fn mcp_list_approvals(state: State<'_, AppState>) -> AppResult<Vec<McpApprovalDto>> {
    state.with_session_no_touch(|_| {
        Ok(state
            .mcp
            .pending_approvals()
            .iter()
            .map(McpApprovalDto::from_record)
            .collect())
    })
}

#[tauri::command]
pub async fn mcp_get_approval(
    state: State<'_, AppState>,
    approval_id: String,
) -> AppResult<McpApprovalStatus> {
    state.with_session_no_touch(|_| {
        state
            .mcp
            .approval_status(&approval_id)
            .ok_or_else(|| AppError::NotFound)
    })
}

#[tauri::command]
pub async fn mcp_approve(
    state: State<'_, AppState>,
    approval_id: String,
) -> AppResult<McpApprovalResult> {
    let record = state
        .mcp
        .take_pending_approval(&approval_id)
        .ok_or_else(|| {
            AppError::Validation("The MCP proposal expired or was already handled.".into())
        })?;
    if record.is_expired() {
        return Err(AppError::Validation(
            "The MCP proposal expired before it could be approved.".into(),
        ));
    }
    let backup = record.clone();
    let result = state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let now = db::now();
        let tx = session.conn.transaction()?;
        let proposal = AgentProposalRecord {
            id: record.id.clone(),
            engagement_id: record.engagement_id.clone(),
            skill: record.skill,
            target_id: record.target_id.clone(),
            payload: record.payload.clone(),
            expires_at: Instant::now(),
        };
        let applied = apply_proposal(&tx, &proposal, &session.profile.display_name, &now)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&record.engagement_id),
                actor_id: Some(&session.profile.id),
                action: "mcp.approval.approved",
                entity_type: "mcp_approval",
                entity_id: Some(&record.id),
                metadata: serde_json::json!({
                    "skill": record.skill.as_str(),
                    "targetId": record.target_id,
                    "createdTestCases": applied.1,
                    "createdFindings": applied.2,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(McpApprovalResult {
            message: applied.0,
            created_test_cases: applied.1,
            created_findings: applied.2,
            updated_engagement: matches!(
                record.skill,
                crate::ai::agent::AgentSkill::ApplicationSummary
                    | crate::ai::agent::AgentSkill::AttackSurface
                    | crate::ai::agent::AgentSkill::Assumptions
            ),
        })
    });
    match result {
        Ok(result) => {
            state.mcp.complete_approval(&record, result.clone());
            Ok(result)
        }
        Err(error) => {
            state.mcp.restore_approval(backup);
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn mcp_reject(
    state: State<'_, AppState>,
    approval_id: String,
) -> AppResult<McpApprovalStatus> {
    let record = state
        .mcp
        .take_pending_approval(&approval_id)
        .ok_or_else(|| {
            AppError::Validation("The MCP proposal expired or was already handled.".into())
        })?;
    let result = state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let now = db::now();
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&record.engagement_id),
                actor_id: Some(&session.profile.id),
                action: "mcp.approval.rejected",
                entity_type: "mcp_approval",
                entity_id: Some(&record.id),
                metadata: serde_json::json!({
                    "skill": record.skill.as_str(),
                    "targetId": record.target_id,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(())
    });
    match result {
        Ok(()) => {
            state.mcp.reject_approval(&record);
            state
                .mcp
                .approval_status(&record.id)
                .ok_or(AppError::NotFound)
        }
        Err(error) => {
            state.mcp.restore_approval(record);
            Err(error)
        }
    }
}

fn ensure_token(session: &mut UnlockedSession) -> AppResult<Zeroizing<String>> {
    if db::settings::get(&session.conn, TOKEN_KEY)?.is_some() {
        return load_token(session);
    }
    let token = Zeroizing::new(URL_SAFE_NO_PAD.encode(crypto::random_bytes::<32>()));
    let encrypted = crypto::encrypt_secret(
        &session.master_key,
        token_aad(&session.profile.id).as_bytes(),
        token.as_bytes(),
    )?;
    let now = db::now();
    let tx = session.conn.transaction()?;
    db::settings::set(&tx, TOKEN_KEY, &encrypted)?;
    audit::append(
        &tx,
        audit::AuditEntry {
            engagement_id: None,
            actor_id: Some(&session.profile.id),
            action: "mcp.token.created",
            entity_type: "mcp",
            entity_id: None,
            metadata: serde_json::json!({}),
            created_at: &now,
        },
    )?;
    tx.commit()?;
    Ok(token)
}

fn load_token(session: &UnlockedSession) -> AppResult<Zeroizing<String>> {
    let encrypted = db::settings::get(&session.conn, TOKEN_KEY)?
        .ok_or_else(|| AppError::Validation("The MCP token has not been generated yet.".into()))?;
    let bytes = crypto::decrypt_secret(
        &session.master_key,
        token_aad(&session.profile.id).as_bytes(),
        &encrypted,
    )?;
    let token = String::from_utf8(bytes.to_vec()).map_err(|_| AppError::Crypto)?;
    Ok(Zeroizing::new(token))
}

fn token_aad(profile_id: &str) -> String {
    format!("testtracker-mcp-token-v1:{profile_id}")
}

fn validate_port(port: u16) -> AppResult<u16> {
    if (1024..=u16::MAX).contains(&port) {
        Ok(port)
    } else {
        Err(AppError::Validation(
            "The MCP port must be between 1024 and 65535.".into(),
        ))
    }
}
