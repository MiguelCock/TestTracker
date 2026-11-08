use tauri::State;

use crate::db;
use crate::domain::models::{DeleteResult, Finding, FindingInput};
use crate::domain::rbac;
use crate::domain::severity::severity_for;
use crate::error::{AppError, AppResult};
use crate::security::audit;
use crate::state::AppState;

#[tauri::command]
pub async fn list_findings(
    state: State<'_, AppState>,
    engagement_id: String,
) -> AppResult<Vec<Finding>> {
    state.with_session(|session| db::findings::list(&session.conn, &engagement_id))
}

#[tauri::command]
pub async fn create_finding(state: State<'_, AppState>, input: FindingInput) -> AppResult<Finding> {
    validate(&input)?;
    let severity = severity_for(input.probability, input.impact_score)?;
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let actor = session.profile.id.clone();
        let now = db::now();
        let tx = session.conn.transaction()?;
        db::test_cases::ensure_engagement(&tx, &input.engagement_id)?;
        db::test_cases::validate_links(&tx, &input.engagement_id, &input.test_case_ids)?;
        let code = if input.code.trim().is_empty() || input.code.trim() == "F-" {
            db::findings::next_code(&tx, &input.engagement_id)?
        } else {
            input.code.trim().to_string()
        };
        let finding = db::findings::create(&tx, &input, severity, &code, &now)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&input.engagement_id),
                actor_id: Some(&actor),
                action: "finding.created",
                entity_type: "finding",
                entity_id: Some(&finding.id),
                metadata: serde_json::json!({
                    "code": finding.code,
                    "severity": severity.as_str(),
                    "status": finding.status.as_str(),
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(finding)
    })
}

#[tauri::command]
pub async fn update_finding(
    state: State<'_, AppState>,
    id: String,
    input: FindingInput,
) -> AppResult<Finding> {
    validate(&input)?;
    let severity = severity_for(input.probability, input.impact_score)?;
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let existing = db::findings::get(&session.conn, &id)?;
        if existing.engagement_id != input.engagement_id {
            return Err(AppError::Validation(
                "The finding does not belong to this engagement.".into(),
            ));
        }
        db::test_cases::validate_links(&session.conn, &input.engagement_id, &input.test_case_ids)?;
        let actor = session.profile.id.clone();
        let now = db::now();
        let tx = session.conn.transaction()?;
        let finding = db::findings::update(&tx, &id, &input, severity, &now)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&input.engagement_id),
                actor_id: Some(&actor),
                action: "finding.updated",
                entity_type: "finding",
                entity_id: Some(&finding.id),
                metadata: serde_json::json!({
                    "severity": severity.as_str(),
                    "status": finding.status.as_str(),
                    "retestStatus": finding.retest_status.as_str(),
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(finding)
    })
}

#[tauri::command]
pub async fn delete_finding(state: State<'_, AppState>, id: String) -> AppResult<DeleteResult> {
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let finding = db::findings::get(&session.conn, &id)?;
        let actor = session.profile.id.clone();
        let now = db::now();
        let tx = session.conn.transaction()?;
        db::findings::delete(&tx, &id)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&finding.engagement_id),
                actor_id: Some(&actor),
                action: "finding.deleted",
                entity_type: "finding",
                entity_id: Some(&id),
                metadata: serde_json::json!({ "code": finding.code, "title": finding.title }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(DeleteResult { id })
    })
}

fn validate(input: &FindingInput) -> AppResult<()> {
    if input.engagement_id.trim().is_empty() {
        return Err(AppError::Validation("Engagement is required.".into()));
    }
    let title = input.title.trim();
    if title.is_empty() || title.chars().count() > 300 {
        return Err(AppError::Validation(
            "Finding title must be between 1 and 300 characters.".into(),
        ));
    }
    if input.probability > 9 || input.impact_score > 9 {
        return Err(AppError::Validation(
            "Probability and impact must be between 0 and 9.".into(),
        ));
    }
    if input.description.chars().count() > 50_000
        || input.impact.chars().count() > 50_000
        || input.remediation.chars().count() > 50_000
    {
        return Err(AppError::Validation(
            "Finding text fields must be at most 50,000 characters.".into(),
        ));
    }
    if input.code.chars().count() > 40
        || input.owasp.chars().count() > 200
        || input.cwe.chars().count() > 200
        || input.owner.chars().count() > 120
    {
        return Err(AppError::Validation("A finding field is too long.".into()));
    }
    if input.test_case_ids.len() > 200 {
        return Err(AppError::Validation("Too many linked test cases.".into()));
    }
    Ok(())
}
