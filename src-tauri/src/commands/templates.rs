use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::db;
use crate::domain::enums::TestCaseStatus;
use crate::domain::models::{
    DeleteResult, TemplateBundleApplyResult, TemplateBundleExport, TemplateBundleImportResult,
    TestCaseInput, TestTemplateBundle,
};
use crate::domain::rbac;
use crate::error::{AppError, AppResult};
use crate::security::audit;
use crate::state::AppState;

const MAX_IMPORT_BYTES: usize = 2 * 1024 * 1024;
const MAX_TEMPLATES: usize = 500;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleFile {
    format: String,
    format_version: u32,
    content_hash: String,
    bundle: BundleFileMeta,
    templates: Vec<BundleFileTemplate>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleFileMeta {
    id: String,
    name: String,
    description: String,
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleFileTemplate {
    id: String,
    domain: String,
    category: String,
    title: String,
    approach: String,
    expected_result: String,
    references: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BundleHashPayload<'a> {
    bundle: &'a BundleFileMeta,
    templates: &'a [BundleFileTemplate],
}

#[tauri::command]
pub async fn list_test_bundles(state: State<'_, AppState>) -> AppResult<Vec<TestTemplateBundle>> {
    state.with_session(|session| db::templates::list_bundles(&session.conn))
}

#[tauri::command]
pub async fn create_test_cases_from_bundle(
    state: State<'_, AppState>,
    engagement_id: String,
    bundle_id: String,
) -> AppResult<TemplateBundleApplyResult> {
    state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        db::test_cases::ensure_engagement(&session.conn, &engagement_id)?;
        db::templates::get_bundle(&session.conn, &bundle_id)?;
        let template_ids = db::templates::bundle_template_ids(&session.conn, &bundle_id)?;
        if template_ids.is_empty() {
            return Err(AppError::Validation(
                "This template bundle has no templates.".into(),
            ));
        }
        let existing = db::test_cases::list(&session.conn, &engagement_id)?;
        let mut used: std::collections::HashSet<String> = existing
            .iter()
            .filter_map(|test| test.template_id.clone())
            .collect();
        let base_order = existing
            .iter()
            .map(|test| test.sort_order)
            .max()
            .unwrap_or(-1)
            + 1;
        let actor = session.profile.id.clone();
        let assignee = session.profile.display_name.clone();
        let now = db::now();
        let tx = session.conn.transaction()?;
        let mut created = Vec::new();
        let mut skipped = 0usize;
        for template_id in template_ids {
            if !used.insert(template_id.clone()) {
                skipped += 1;
                continue;
            }
            let template = db::test_cases::get_template(&tx, &template_id)?;
            let notes = if template.expected_result.is_empty() {
                String::new()
            } else {
                format!(
                    "Expected result: {}\nReferences: {}",
                    template.expected_result, template.references
                )
            };
            let input = TestCaseInput {
                engagement_id: engagement_id.clone(),
                template_id: Some(template.id),
                title: template.title,
                approach: template.approach,
                category: template.category,
                status: TestCaseStatus::NotStarted,
                result: String::new(),
                notes,
                assignee: assignee.clone(),
                sort_order: base_order + created.len() as i64,
            };
            created.push(db::test_cases::create(&tx, &input, &now)?);
        }
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&engagement_id),
                actor_id: Some(&actor),
                action: "test_case.bundle_created",
                entity_type: "engagement",
                entity_id: Some(&engagement_id),
                metadata: serde_json::json!({
                    "bundleId": bundle_id,
                    "created": created.len(),
                    "skipped": skipped,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(TemplateBundleApplyResult {
            created: created.len(),
            skipped,
            test_cases: created,
        })
    })
}

#[tauri::command]
pub async fn export_template_bundle(
    state: State<'_, AppState>,
    bundle_id: String,
) -> AppResult<TemplateBundleExport> {
    state.with_session(|session| {
        let bundle = db::templates::get_bundle(&session.conn, &bundle_id)?;
        let items = db::templates::bundle_export_items(&session.conn, &bundle_id)?;
        let templates = items
            .into_iter()
            .map(|(local_id, origin_id)| {
                let template = db::test_cases::get_template(&session.conn, &local_id)?;
                Ok(BundleFileTemplate {
                    id: if origin_id.is_empty() {
                        template.id
                    } else {
                        origin_id
                    },
                    domain: template.domain,
                    category: template.category,
                    title: template.title,
                    approach: template.approach,
                    expected_result: template.expected_result,
                    references: template.references,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        let meta = BundleFileMeta {
            id: bundle
                .origin_id
                .clone()
                .unwrap_or_else(|| bundle.id.clone()),
            name: bundle.name.clone(),
            description: bundle.description.clone(),
            version: bundle.version.clone(),
        };
        let hash = bundle_hash(&meta, &templates)?;
        let file = BundleFile {
            format: "testtracker-template-bundle".into(),
            format_version: 1,
            content_hash: hash.clone(),
            bundle: meta,
            templates,
        };
        let json = serde_json::to_string_pretty(&file)?;
        let filename = format!("{}.tttemplates.json", sanitize(&bundle.id));
        let actor = session.profile.id.clone();
        let now = db::now();
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&actor),
                action: "template_bundle.exported",
                entity_type: "template_bundle",
                entity_id: Some(&bundle.id),
                metadata: serde_json::json!({
                    "name": bundle.name,
                    "templateCount": file.templates.len(),
                    "contentHash": hash,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(TemplateBundleExport {
            filename,
            template_count: file.templates.len(),
            content_hash: hash,
            json,
        })
    })
}

#[tauri::command]
pub async fn import_template_bundle(
    state: State<'_, AppState>,
    json: String,
) -> AppResult<TemplateBundleImportResult> {
    if json.len() > MAX_IMPORT_BYTES {
        return Err(AppError::Validation(
            "Template bundle files must be 2 MB or smaller.".into(),
        ));
    }
    let file: BundleFile = serde_json::from_str(&json)
        .map_err(|_| AppError::Validation("Invalid template bundle JSON.".into()))?;
    if file.format != "testtracker-template-bundle" || file.format_version != 1 {
        return Err(AppError::Validation(
            "Unsupported template bundle format.".into(),
        ));
    }
    validate_import(&file)?;
    let expected_hash = bundle_hash(&file.bundle, &file.templates)?;
    if expected_hash != file.content_hash {
        return Err(AppError::Validation(
            "Template bundle integrity check failed.".into(),
        ));
    }
    state.with_session(|session| {
        if !rbac::can_manage(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let now = db::now();
        let tx = session.conn.transaction()?;
        let existing = db::templates::find_imported_by_origin(&tx, &file.bundle.id)?;
        let updated = existing.is_some();
        let local_bundle_id = existing.unwrap_or_else(|| format!("imp-{}", Uuid::now_v7()));
        tx.execute(
            "INSERT INTO test_template_bundles
                (id, name, description, version, source, origin_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'imported', ?5, ?6, ?6)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                version = excluded.version,
                origin_id = excluded.origin_id,
                updated_at = excluded.updated_at",
            rusqlite::params![
                local_bundle_id,
                file.bundle.name,
                file.bundle.description,
                file.bundle.version,
                file.bundle.id,
                now,
            ],
        )?;
        tx.execute(
            "DELETE FROM test_template_bundle_items WHERE bundle_id = ?1",
            [&local_bundle_id],
        )?;
        let mut insert_template = tx.prepare(
            "INSERT INTO test_case_templates
                (id, domain, category, title, approach, expected_result, references_text, source, origin_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'imported', ?8)
             ON CONFLICT(id) DO UPDATE SET
                domain = excluded.domain,
                category = excluded.category,
                title = excluded.title,
                approach = excluded.approach,
                expected_result = excluded.expected_result,
                references_text = excluded.references_text,
                source = 'imported',
                origin_id = excluded.origin_id
             WHERE test_case_templates.source = 'imported'",
        )?;
        let mut insert_item = tx.prepare(
            "INSERT INTO test_template_bundle_items
                (bundle_id, template_id, origin_template_id, sort_order)
             VALUES (?1, ?2, ?3, ?4)",
        )?;
        for (index, template) in file.templates.iter().enumerate() {
            let local_template_id = format!(
                "imp-{}-{}",
                local_bundle_id.replace("imp-", "").chars().take(8).collect::<String>(),
                &blake3::hash(template.id.as_bytes()).to_hex().to_string()[..12]
            );
            insert_template.execute(rusqlite::params![
                local_template_id,
                template.domain,
                template.category,
                template.title,
                template.approach,
                template.expected_result,
                template.references,
                template.id,
            ])?;
            insert_item.execute(rusqlite::params![
                local_bundle_id,
                local_template_id,
                template.id,
                index as i64,
            ])?;
        }
        drop(insert_item);
        drop(insert_template);
        db::templates::delete_orphan_imported_templates(&tx)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&session.profile.id),
                action: "template_bundle.imported",
                entity_type: "template_bundle",
                entity_id: Some(&local_bundle_id),
                metadata: serde_json::json!({
                    "name": file.bundle.name,
                    "originId": file.bundle.id,
                    "updated": updated,
                    "templateCount": file.templates.len(),
                    "contentHash": expected_hash,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(TemplateBundleImportResult {
            bundle_id: local_bundle_id,
            name: file.bundle.name,
            version: file.bundle.version,
            template_count: file.templates.len(),
            updated,
        })
    })
}

#[tauri::command]
pub async fn delete_template_bundle(
    state: State<'_, AppState>,
    bundle_id: String,
) -> AppResult<DeleteResult> {
    state.with_session(|session| {
        if !rbac::can_manage(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let now = db::now();
        let tx = session.conn.transaction()?;
        db::templates::delete_imported_bundle(&tx, &bundle_id)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&session.profile.id),
                action: "template_bundle.deleted",
                entity_type: "template_bundle",
                entity_id: Some(&bundle_id),
                metadata: serde_json::json!({}),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(DeleteResult { id: bundle_id })
    })
}

fn validate_import(file: &BundleFile) -> AppResult<()> {
    if file.templates.is_empty() || file.templates.len() > MAX_TEMPLATES {
        return Err(AppError::Validation(
            "Template bundles must contain between 1 and 500 templates.".into(),
        ));
    }
    if file.bundle.id.trim().is_empty()
        || file.bundle.id.chars().count() > 120
        || file.bundle.name.trim().is_empty()
        || file.bundle.name.chars().count() > 200
        || file.bundle.version.chars().count() > 40
    {
        return Err(AppError::Validation(
            "Template bundle metadata is invalid.".into(),
        ));
    }
    let mut ids = std::collections::HashSet::new();
    for template in &file.templates {
        if template.id.trim().is_empty()
            || template.id.chars().count() > 160
            || !ids.insert(template.id.clone())
        {
            return Err(AppError::Validation(
                "Template IDs must be unique and valid.".into(),
            ));
        }
        if template.title.trim().is_empty()
            || template.title.chars().count() > 300
            || template.domain.trim().is_empty()
            || template.domain.chars().count() > 80
            || template.category.chars().count() > 120
            || template.approach.chars().count() > 20_000
            || template.expected_result.chars().count() > 10_000
            || template.references.chars().count() > 1_000
        {
            return Err(AppError::Validation(
                "A template field is invalid or too long.".into(),
            ));
        }
    }
    Ok(())
}

fn bundle_hash(meta: &BundleFileMeta, templates: &[BundleFileTemplate]) -> AppResult<String> {
    let payload = BundleHashPayload {
        bundle: meta,
        templates,
    };
    Ok(blake3::hash(&serde_json::to_vec(&payload)?)
        .to_hex()
        .to_string())
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template(id: &str) -> BundleFileTemplate {
        BundleFileTemplate {
            id: id.into(),
            domain: "web".into(),
            category: "Authorization".into(),
            title: "Synthetic check".into(),
            approach: "Review access.".into(),
            expected_result: "Access is denied.".into(),
            references: "CWE-285".into(),
        }
    }

    #[test]
    fn validates_and_hashes_bundle_files() {
        let meta = BundleFileMeta {
            id: "team-web".into(),
            name: "Team web".into(),
            description: "Synthetic".into(),
            version: "1.0.0".into(),
        };
        let templates = vec![template("web-auth"), template("web-access")];
        let hash = bundle_hash(&meta, &templates).expect("hash");
        let file = BundleFile {
            format: "testtracker-template-bundle".into(),
            format_version: 1,
            content_hash: hash.clone(),
            bundle: meta,
            templates,
        };
        assert!(validate_import(&file).is_ok());
        assert_eq!(bundle_hash(&file.bundle, &file.templates).unwrap(), hash);
    }

    #[test]
    fn rejects_duplicate_template_ids() {
        let file = BundleFile {
            format: "testtracker-template-bundle".into(),
            format_version: 1,
            content_hash: String::new(),
            bundle: BundleFileMeta {
                id: "dup".into(),
                name: "Dup".into(),
                description: String::new(),
                version: "1".into(),
            },
            templates: vec![template("x"), template("x")],
        };
        assert!(validate_import(&file).is_err());
    }
}
