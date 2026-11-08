use tauri::State;
use uuid::Uuid;

use crate::db;
use crate::domain::models::{
    AuditEvent, DashboardMetrics, DeleteResult, Engagement, EngagementDetail, EngagementInput,
    EngagementSummary,
};
use crate::domain::rbac;
use crate::error::{AppError, AppResult};
use crate::security::audit;
use crate::state::AppState;

#[tauri::command]
pub async fn list_engagements(state: State<'_, AppState>) -> AppResult<Vec<EngagementSummary>> {
    state.with_session(|session| db::engagements::list(&session.conn))
}

#[tauri::command]
pub async fn get_engagement(state: State<'_, AppState>, id: String) -> AppResult<EngagementDetail> {
    state.with_session(|session| db::engagements::detail(&session.conn, &id))
}

#[tauri::command]
pub async fn create_engagement(
    state: State<'_, AppState>,
    input: EngagementInput,
) -> AppResult<Engagement> {
    validate_engagement(&input)?;
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let actor = session.profile.id.clone();
        let now = db::now();
        let tx = session.conn.transaction()?;
        let id = Uuid::now_v7().to_string();
        let engagement = db::engagements::insert(&tx, &id, &input, &actor, &now)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&id),
                actor_id: Some(&actor),
                action: "engagement.created",
                entity_type: "engagement",
                entity_id: Some(&id),
                metadata: serde_json::json!({
                    "name": engagement.name,
                    "status": engagement.status.as_str(),
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(engagement)
    })
}

#[tauri::command]
pub async fn update_engagement(
    state: State<'_, AppState>,
    id: String,
    input: EngagementInput,
) -> AppResult<Engagement> {
    validate_engagement(&input)?;
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let actor = session.profile.id.clone();
        let now = db::now();
        let tx = session.conn.transaction()?;
        let engagement = db::engagements::update(&tx, &id, &input, &now)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&id),
                actor_id: Some(&actor),
                action: "engagement.updated",
                entity_type: "engagement",
                entity_id: Some(&id),
                metadata: serde_json::json!({ "version": engagement.version }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(engagement)
    })
}

#[tauri::command]
pub async fn delete_engagement(state: State<'_, AppState>, id: String) -> AppResult<DeleteResult> {
    let data_dir = state.data_dir.clone();
    let result = state.with_session(|session| {
        if !rbac::can_manage(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let actor = session.profile.id.clone();
        let now = db::now();
        let tx = session.conn.transaction()?;
        let engagement = db::engagements::get(&tx, &id)?;
        db::engagements::delete(&tx, &id)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&id),
                actor_id: Some(&actor),
                action: "engagement.deleted",
                entity_type: "engagement",
                entity_id: Some(&id),
                metadata: serde_json::json!({ "name": engagement.name }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(DeleteResult { id: id.clone() })
    })?;
    let _ = std::fs::remove_dir_all(data_dir.join("evidence").join(&id));
    let _ = std::fs::remove_dir_all(data_dir.join("documents").join(&id));
    Ok(result)
}

#[tauri::command]
pub async fn dashboard_metrics(state: State<'_, AppState>) -> AppResult<DashboardMetrics> {
    state.with_session(|session| db::metrics::dashboard(&session.conn))
}

#[tauri::command]
pub async fn list_audit_events(
    state: State<'_, AppState>,
    engagement_id: Option<String>,
    limit: Option<i64>,
) -> AppResult<Vec<AuditEvent>> {
    state.with_session(|session| {
        db::audit_repo::list(
            &session.conn,
            engagement_id.as_deref(),
            limit.unwrap_or(100),
        )
    })
}

#[tauri::command]
pub async fn verify_audit_chain(state: State<'_, AppState>) -> AppResult<bool> {
    state.with_session(|session| audit::verify_chain(&session.conn))
}

fn validate_engagement(input: &EngagementInput) -> AppResult<()> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 200 {
        return Err(AppError::Validation(
            "Engagement name must be between 1 and 200 characters.".into(),
        ));
    }
    if input.client_code.chars().count() > 120 {
        return Err(AppError::Validation(
            "Client code must be at most 120 characters.".into(),
        ));
    }
    validate_date(&input.start_date, "start date")?;
    validate_date(&input.end_date, "end date")?;
    if !input.start_date.is_empty() && !input.end_date.is_empty() {
        let start = chrono::NaiveDate::parse_from_str(&input.start_date, "%Y-%m-%d");
        let end = chrono::NaiveDate::parse_from_str(&input.end_date, "%Y-%m-%d");
        if let (Ok(start), Ok(end)) = (start, end)
            && start > end
        {
            return Err(AppError::Validation(
                "Start date must not be after end date.".into(),
            ));
        }
    }
    if input.application_summary.chars().count() > 20_000
        || input.attack_surface.chars().count() > 20_000
    {
        return Err(AppError::Validation(
            "Summary fields must be at most 20,000 characters.".into(),
        ));
    }
    if input.scope_assets.len() > 500
        || input.risks.len() > 200
        || input.assumptions.len() > 200
        || input.contacts.len() > 100
    {
        return Err(AppError::Validation(
            "Too many child records in the engagement.".into(),
        ));
    }
    for asset in &input.scope_assets {
        if asset.kind.trim().is_empty() || asset.value.trim().is_empty() {
            return Err(AppError::Validation(
                "Scope assets require a kind and value.".into(),
            ));
        }
        if asset.kind.chars().count() > 80 || asset.value.chars().count() > 500 {
            return Err(AppError::Validation(
                "Scope asset fields are too long.".into(),
            ));
        }
    }
    for risk in &input.risks {
        if risk.title.trim().is_empty() || risk.title.chars().count() > 200 {
            return Err(AppError::Validation(
                "Risk titles must be between 1 and 200 characters.".into(),
            ));
        }
        if risk.likelihood > 9 || risk.impact > 9 {
            return Err(AppError::Validation(
                "Risk likelihood and impact must be between 0 and 9.".into(),
            ));
        }
    }
    for text in &input.assumptions {
        if text.chars().count() > 2_000 {
            return Err(AppError::Validation("Assumption text is too long.".into()));
        }
    }
    for contact in &input.contacts {
        if contact.name.trim().is_empty() || contact.name.chars().count() > 200 {
            return Err(AppError::Validation(
                "Contact names must be between 1 and 200 characters.".into(),
            ));
        }
        if contact.email.chars().count() > 320 {
            return Err(AppError::Validation("Contact email is too long.".into()));
        }
    }
    Ok(())
}

fn validate_date(value: &str, label: &str) -> AppResult<()> {
    if value.is_empty() {
        return Ok(());
    }
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| AppError::Validation(format!("Invalid {label}; use YYYY-MM-DD.")))
}
