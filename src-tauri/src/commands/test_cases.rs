use tauri::State;

use crate::db;
use crate::domain::enums::TestCaseStatus;
use crate::domain::models::{DeleteResult, TestCase, TestCaseInput, TestCaseTemplate};
use crate::domain::rbac;
use crate::error::{AppError, AppResult};
use crate::security::audit;
use crate::state::AppState;

#[tauri::command]
pub async fn list_test_cases(
    state: State<'_, AppState>,
    engagement_id: String,
) -> AppResult<Vec<TestCase>> {
    state.with_session(|session| db::test_cases::list(&session.conn, &engagement_id))
}

#[tauri::command]
pub async fn list_test_case_templates(
    state: State<'_, AppState>,
    domain: Option<String>,
) -> AppResult<Vec<TestCaseTemplate>> {
    state.with_session(|session| db::test_cases::templates(&session.conn, domain.as_deref()))
}

#[tauri::command]
pub async fn create_test_case(
    state: State<'_, AppState>,
    input: TestCaseInput,
) -> AppResult<TestCase> {
    validate(&input)?;
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        if let Some(template_id) = input.template_id.as_deref()
            && !db::test_cases::template_exists(&session.conn, template_id)?
        {
            return Err(AppError::Validation(
                "The selected template does not exist.".into(),
            ));
        }
        let actor = session.profile.id.clone();
        let now = db::now();
        let mut input = input;
        input.assignee = session.profile.display_name.clone();
        let tx = session.conn.transaction()?;
        let test_case = db::test_cases::create(&tx, &input, &now)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&input.engagement_id),
                actor_id: Some(&actor),
                action: "test_case.created",
                entity_type: "test_case",
                entity_id: Some(&test_case.id),
                metadata: serde_json::json!({ "title": test_case.title }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(test_case)
    })
}

#[tauri::command]
pub async fn update_test_case(
    state: State<'_, AppState>,
    id: String,
    input: TestCaseInput,
) -> AppResult<TestCase> {
    validate(&input)?;
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        if let Some(template_id) = input.template_id.as_deref()
            && !db::test_cases::template_exists(&session.conn, template_id)?
        {
            return Err(AppError::Validation(
                "The selected template does not exist.".into(),
            ));
        }
        let actor = session.profile.id.clone();
        let now = db::now();
        let mut input = input;
        input.assignee = session.profile.display_name.clone();
        let tx = session.conn.transaction()?;
        let test_case = db::test_cases::update(&tx, &id, &input, &now)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&input.engagement_id),
                actor_id: Some(&actor),
                action: "test_case.updated",
                entity_type: "test_case",
                entity_id: Some(&test_case.id),
                metadata: serde_json::json!({
                    "status": test_case.status.as_str(),
                    "hasFinding": false,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(test_case)
    })
}

#[tauri::command]
pub async fn delete_test_case(state: State<'_, AppState>, id: String) -> AppResult<DeleteResult> {
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let test_case = db::test_cases::get(&session.conn, &id)?;
        let actor = session.profile.id.clone();
        let now = db::now();
        let tx = session.conn.transaction()?;
        db::test_cases::delete(&tx, &id)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&test_case.engagement_id),
                actor_id: Some(&actor),
                action: "test_case.deleted",
                entity_type: "test_case",
                entity_id: Some(&id),
                metadata: serde_json::json!({ "title": test_case.title }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(DeleteResult { id })
    })
}

#[tauri::command]
pub async fn create_test_cases_from_templates(
    state: State<'_, AppState>,
    engagement_id: String,
    template_ids: Vec<String>,
) -> AppResult<Vec<TestCase>> {
    if template_ids.is_empty() || template_ids.len() > 200 {
        return Err(AppError::Validation(
            "Select between 1 and 200 templates.".into(),
        ));
    }
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        db::test_cases::ensure_engagement(&session.conn, &engagement_id)?;
        let actor = session.profile.id.clone();
        let now = db::now();
        let tx = session.conn.transaction()?;
        let mut created = Vec::new();
        for (index, template_id) in template_ids.iter().enumerate() {
            let template = db::test_cases::get_template(&tx, template_id)?;
            let input = TestCaseInput {
                engagement_id: engagement_id.clone(),
                template_id: Some(template.id),
                title: template.title,
                approach: template.approach,
                category: template.category,
                status: TestCaseStatus::NotStarted,
                result: String::new(),
                notes: String::new(),
                assignee: session.profile.display_name.clone(),
                sort_order: index as i64,
            };
            created.push(db::test_cases::create(&tx, &input, &now)?);
        }
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&engagement_id),
                actor_id: Some(&actor),
                action: "test_case.bulk_created",
                entity_type: "engagement",
                entity_id: Some(&engagement_id),
                metadata: serde_json::json!({ "count": created.len() }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(created)
    })
}

#[tauri::command]
pub async fn reorder_test_cases(
    state: State<'_, AppState>,
    engagement_id: String,
    ordered_ids: Vec<String>,
) -> AppResult<Vec<TestCase>> {
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let now = db::now();
        let tx = session.conn.transaction()?;
        db::test_cases::reorder(&tx, &engagement_id, &ordered_ids, &now)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&engagement_id),
                actor_id: Some(&session.profile.id),
                action: "test_case.reordered",
                entity_type: "engagement",
                entity_id: Some(&engagement_id),
                metadata: serde_json::json!({ "count": ordered_ids.len() }),
                created_at: &now,
            },
        )?;
        let result = db::test_cases::list(&tx, &engagement_id)?;
        tx.commit()?;
        Ok(result)
    })
}

pub(crate) fn validate(input: &TestCaseInput) -> AppResult<()> {
    if input.engagement_id.trim().is_empty() {
        return Err(AppError::Validation("Engagement is required.".into()));
    }
    let title = input.title.trim();
    if title.is_empty() || title.chars().count() > 300 {
        return Err(AppError::Validation(
            "Test case title must be between 1 and 300 characters.".into(),
        ));
    }
    if input.approach.chars().count() > 20_000
        || input.result.chars().count() > 20_000
        || input.notes.chars().count() > 20_000
    {
        return Err(AppError::Validation(
            "Test case text fields must be at most 20,000 characters.".into(),
        ));
    }
    if input.category.chars().count() > 120 || input.assignee.chars().count() > 120 {
        return Err(AppError::Validation(
            "Category or assignee is too long.".into(),
        ));
    }
    Ok(())
}
