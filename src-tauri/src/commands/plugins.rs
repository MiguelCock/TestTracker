use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use tauri::State;
use uuid::Uuid;

use crate::db;
use crate::domain::models::DeleteResult;
use crate::domain::rbac;
use crate::error::{AppError, AppResult};
use crate::plugins::model::{
    AddPluginPublisherInput, CAPABILITY_ENGAGEMENT_READ, PluginManifest, PluginPublisher,
    PluginResult, PluginRunResult, PluginSummary,
};
use crate::plugins::{package, runtime};
use crate::security::{audit, identity};
use crate::state::AppState;

const MAX_INSTALL_BASE64_BYTES: usize = 12 * 1024 * 1024;

#[tauri::command]
pub async fn list_plugin_publishers(state: State<'_, AppState>) -> AppResult<Vec<PluginPublisher>> {
    state.with_session(|session| db::plugins::list_publishers(&session.conn))
}

#[tauri::command]
pub async fn add_plugin_publisher(
    state: State<'_, AppState>,
    input: AddPluginPublisherInput,
) -> AppResult<PluginPublisher> {
    let id = input.id.trim();
    let name = input.name.trim();
    let public_key = input.public_key.trim();
    crate::plugins::model::validate_plugin_id(id, "publisher id")?;
    if name.is_empty() || name.chars().count() > 120 {
        return Err(AppError::Validation(
            "Publisher name must be between 1 and 120 characters.".into(),
        ));
    }
    if !identity::validate_public_key(public_key) {
        return Err(AppError::Validation(
            "Enter a valid Ed25519 public key in hexadecimal.".into(),
        ));
    }
    state.with_session(|session| {
        if !rbac::can_manage(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let now = db::now();
        let tx = session.conn.transaction()?;
        db::plugins::insert_publisher(&tx, id, name, public_key, &session.profile.id, &now)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&session.profile.id),
                action: "plugin_publisher.added",
                entity_type: "plugin_publisher",
                entity_id: Some(id),
                metadata: serde_json::json!({ "name": name }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(PluginPublisher {
            id: id.to_string(),
            name: name.to_string(),
            public_key: public_key.to_string(),
            created_at: now,
        })
    })
}

#[tauri::command]
pub async fn delete_plugin_publisher(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<DeleteResult> {
    state.with_session(|session| {
        if !rbac::can_manage(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        db::plugins::get_publisher(&session.conn, &id)?;
        let now = db::now();
        let tx = session.conn.transaction()?;
        db::plugins::delete_publisher(&tx, &id)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&session.profile.id),
                action: "plugin_publisher.deleted",
                entity_type: "plugin_publisher",
                entity_id: Some(&id),
                metadata: serde_json::json!({}),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(DeleteResult { id })
    })
}

#[tauri::command]
pub async fn list_plugins(state: State<'_, AppState>) -> AppResult<Vec<PluginSummary>> {
    state.with_session(|session| db::plugins::list(&session.conn))
}

#[tauri::command]
pub async fn install_plugin(
    state: State<'_, AppState>,
    data_base64: String,
) -> AppResult<PluginSummary> {
    if data_base64.len() > MAX_INSTALL_BASE64_BYTES {
        return Err(AppError::Validation(
            "Plugin package is larger than 8 MB.".into(),
        ));
    }
    let bytes = BASE64
        .decode(data_base64.as_bytes())
        .map_err(|_| AppError::Validation("Plugin package is not valid base64.".into()))?;
    let data_dir = state.data_dir.clone();

    state.with_session(|session| {
        if !rbac::can_manage(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let package = package::parse_package(&bytes)?;
        let publisher = db::plugins::get_publisher(&session.conn, &package.manifest.publisher_id)
            .map_err(|_| {
            AppError::Validation(
                "Publisher is not trusted. Add the publisher key in Settings first.".into(),
            )
        })?;
        if !package::verify_signature(&package, &publisher.public_key) {
            return Err(AppError::Validation(
                "Plugin signature verification failed.".into(),
            ));
        }
        runtime::validate_wasm(&package.wasm)?;
        let wasm_hash = blake3::hash(&package.wasm).to_hex().to_string();
        let manifest_json = String::from_utf8(package.manifest_bytes.clone())
            .map_err(|_| AppError::Validation("Invalid plugin manifest.".into()))?;
        let directory = plugins_root(&data_dir).join(&package.manifest.id);
        fs::create_dir_all(&directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
        }
        let file_suffix = &Uuid::now_v7().to_string()[..8];
        let wasm_path = directory.join(format!("{}-{file_suffix}.wasm", package.manifest.version));
        write_private(&wasm_path, &package.wasm)?;
        let old = db::plugins::get(&session.conn, &package.manifest.id).ok();
        let stored = db::plugins::StoredPlugin {
            id: package.manifest.id.clone(),
            version: package.manifest.version.clone(),
            name: package.manifest.name.clone(),
            publisher_id: package.manifest.publisher_id.clone(),
            description: package.manifest.description.clone(),
            capabilities: package.manifest.capabilities.clone(),
            manifest_json,
            wasm_path: wasm_path.to_string_lossy().to_string(),
            wasm_hash,
            enabled: true,
            installed_at: db::now(),
        };
        let now = db::now();
        let result = (|| -> AppResult<PluginSummary> {
            let tx = session.conn.transaction()?;
            db::plugins::upsert(&tx, &stored, &session.profile.id, &now)?;
            audit::append(
                &tx,
                audit::AuditEntry {
                    engagement_id: None,
                    actor_id: Some(&session.profile.id),
                    action: "plugin.installed",
                    entity_type: "plugin",
                    entity_id: Some(&stored.id),
                    metadata: serde_json::json!({
                        "version": stored.version,
                        "publisherId": stored.publisher_id,
                        "capabilities": stored.capabilities,
                    }),
                    created_at: &now,
                },
            )?;
            tx.commit()?;
            Ok(stored.summary())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&wasm_path);
        } else if let Some(old) = old
            && old.wasm_path != stored.wasm_path
        {
            let _ = fs::remove_file(old.wasm_path);
        }
        result
    })
}

#[tauri::command]
pub async fn set_plugin_enabled(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> AppResult<PluginSummary> {
    state.with_session(|session| {
        if !rbac::can_manage(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let now = db::now();
        let tx = session.conn.transaction()?;
        db::plugins::set_enabled(&tx, &id, enabled)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&session.profile.id),
                action: if enabled {
                    "plugin.enabled"
                } else {
                    "plugin.disabled"
                },
                entity_type: "plugin",
                entity_id: Some(&id),
                metadata: serde_json::json!({}),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(db::plugins::get(&session.conn, &id)?.summary())
    })
}

#[tauri::command]
pub async fn delete_plugin(state: State<'_, AppState>, id: String) -> AppResult<DeleteResult> {
    let data_dir = state.data_dir.clone();
    state.with_session(|session| {
        if !rbac::can_manage(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        db::plugins::get(&session.conn, &id)?;
        let now = db::now();
        let tx = session.conn.transaction()?;
        db::plugins::delete(&tx, &id)?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&session.profile.id),
                action: "plugin.deleted",
                entity_type: "plugin",
                entity_id: Some(&id),
                metadata: serde_json::json!({}),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        let _ = fs::remove_dir_all(plugins_root(&data_dir).join(&id));
        Ok(DeleteResult { id })
    })
}

#[tauri::command]
pub async fn run_plugin(
    state: State<'_, AppState>,
    plugin_id: String,
    engagement_id: String,
) -> AppResult<PluginRunResult> {
    let data_dir = state.data_dir.clone();
    let prepared = state.with_session(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let plugin = db::plugins::get(&session.conn, &plugin_id)?;
        if !plugin.enabled {
            return Err(AppError::Validation("This plugin is disabled.".into()));
        }
        let manifest: PluginManifest = serde_json::from_str(&plugin.manifest_json)
            .map_err(|_| AppError::Plugin("Stored plugin manifest is invalid.".into()))?;
        manifest.validate()?;
        let path = validate_plugin_path(&data_dir, &plugin.wasm_path)?;
        let wasm = fs::read(&path)?;
        if blake3::hash(&wasm).to_hex().to_string() != plugin.wasm_hash {
            return Err(AppError::Plugin(
                "Plugin file hash does not match the installed package.".into(),
            ));
        }
        let mut engagement = serde_json::Value::Null;
        let mut test_cases = serde_json::Value::Array(Vec::new());
        let mut findings = serde_json::Value::Array(Vec::new());
        if manifest.has_capability(CAPABILITY_ENGAGEMENT_READ) {
            db::test_cases::ensure_engagement(&session.conn, &engagement_id)?;
            let detail = db::engagements::detail(&session.conn, &engagement_id)?;
            let tests = db::test_cases::list(&session.conn, &engagement_id)?;
            let findings_list = db::findings::list(&session.conn, &engagement_id)?;
            engagement = serde_json::json!({
                "engagement": detail.engagement,
                "scopeAssets": detail.scope_assets,
                "risks": detail.risks,
                "assumptions": detail.assumptions,
                "contacts": detail.contacts,
            });
            test_cases = serde_json::to_value(tests)?;
            findings = serde_json::to_value(findings_list)?;
        }
        let input = serde_json::json!({
            "schema": "testtracker.plugin.input.v1",
            "plugin": { "id": plugin.id, "version": plugin.version },
            "capabilities": manifest.capabilities,
            "engagement": engagement,
            "testCases": test_cases,
            "findings": findings,
        });
        let input = serde_json::to_string(&input)?;
        Ok((plugin, manifest, input, wasm))
    })?;
    let (plugin, _manifest, input, wasm) = prepared;

    let runtime_output = match runtime::run_wasm(&wasm, &input) {
        Ok(output) => output,
        Err(error) => {
            audit_plugin_failure(&state, &plugin.id, &plugin.version, &engagement_id, &error);
            return Err(error);
        }
    };
    let result: PluginResult = serde_json::from_str(&runtime_output.json)
        .map_err(|_| AppError::Plugin("Plugin returned invalid JSON.".into()))?;
    result.validate()?;
    let output_hash = blake3::hash(runtime_output.json.as_bytes())
        .to_hex()
        .to_string();
    state.with_session(|session| {
        let now = db::now();
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(&engagement_id),
                actor_id: Some(&session.profile.id),
                action: "plugin.executed",
                entity_type: "plugin",
                entity_id: Some(&plugin.id),
                metadata: serde_json::json!({
                    "version": plugin.version,
                    "outputHash": output_hash,
                    "fuelUsed": runtime_output.fuel_used,
                    "logCount": runtime_output.logs.len(),
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(())
    })?;
    Ok(PluginRunResult {
        plugin_id: plugin.id,
        plugin_name: plugin.name,
        plugin_version: plugin.version,
        summary: result.summary,
        test_cases: result.test_cases,
        findings: result.findings,
        notes: result.notes,
        logs: runtime_output.logs,
        fuel_used: runtime_output.fuel_used,
    })
}

fn audit_plugin_failure(
    state: &AppState,
    plugin_id: &str,
    version: &str,
    engagement_id: &str,
    error: &AppError,
) {
    let reason = match error {
        AppError::Plugin(message) => message.clone(),
        _ => error.user_message().to_string(),
    };
    let _ = state.with_session(|session| {
        let now = db::now();
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: Some(engagement_id),
                actor_id: Some(&session.profile.id),
                action: "plugin.failed",
                entity_type: "plugin",
                entity_id: Some(plugin_id),
                metadata: serde_json::json!({ "version": version, "reason": reason }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(())
    });
}

fn plugins_root(data_dir: &Path) -> PathBuf {
    data_dir.join("plugins")
}

fn validate_plugin_path(data_dir: &Path, stored: &str) -> AppResult<PathBuf> {
    let path = PathBuf::from(stored);
    if !path.starts_with(plugins_root(data_dir)) {
        return Err(AppError::Plugin("Invalid plugin path.".into()));
    }
    Ok(path)
}

fn write_private(path: &Path, contents: &[u8]) -> AppResult<()> {
    let tmp = path.with_extension("tmp");
    let mut options = fs::OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&tmp)?;
    file.write_all(contents)?;
    file.sync_all()?;
    fs::rename(&tmp, path)?;
    Ok(())
}
