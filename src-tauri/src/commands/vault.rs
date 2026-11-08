use std::fs;
use std::time::Instant;

use tauri::State;
use uuid::Uuid;

use crate::db;
use crate::domain::enums::ProfileRole;
use crate::domain::models::{ProfileSummary, SessionInfo};
use crate::error::{AppError, AppResult};
use crate::security::{audit, identity, vault};
use crate::state::{AppState, UnlockedSession};

#[tauri::command]
pub async fn session_info(state: State<'_, AppState>) -> AppResult<SessionInfo> {
    Ok(state.session_info())
}

#[tauri::command]
pub async fn initialize_vault(
    state: State<'_, AppState>,
    passphrase: String,
    display_name: String,
) -> AppResult<SessionInfo> {
    if state.initialized() {
        return Err(AppError::Conflict("vault already exists".into()));
    }
    vault::validate_passphrase(&passphrase)?;
    let display_name = display_name.trim();
    if display_name.is_empty() || display_name.chars().count() > 120 {
        return Err(AppError::Validation(
            "Display name must be between 1 and 120 characters.".into(),
        ));
    }

    let master_key = vault::create(&state.vault_path, &passphrase)?;
    let result = initialize_data(&state, &master_key, display_name);
    match result {
        Ok(info) => Ok(info),
        Err(error) => {
            let _ = fs::remove_file(&state.vault_path);
            let _ = fs::remove_file(&state.db_path);
            let _ = fs::remove_file(state.db_path.with_extension("db-wal"));
            let _ = fs::remove_file(state.db_path.with_extension("db-shm"));
            let _ = fs::remove_file(state.db_path.with_extension("db-journal"));
            Err(error)
        }
    }
}

fn initialize_data(
    state: &AppState,
    master_key: &zeroize::Zeroizing<[u8; 32]>,
    display_name: &str,
) -> AppResult<SessionInfo> {
    let mut conn = db::open_encrypted(&state.db_path, master_key)?;
    db::apply_migrations(&mut conn)?;

    let profile_id = Uuid::now_v7().to_string();
    let signing_key = identity::generate();
    let (public_key, encrypted_private_key) =
        identity::encrypt_signing_key(master_key, &profile_id, &signing_key)?;
    let now = db::now();
    db::profiles::insert(
        &conn,
        &profile_id,
        display_name,
        ProfileRole::Admin,
        &public_key,
        &encrypted_private_key,
        &now,
    )?;
    audit::append(
        &conn,
        audit::AuditEntry {
            engagement_id: None,
            actor_id: Some(&profile_id),
            action: "vault.initialized",
            entity_type: "vault",
            entity_id: None,
            metadata: serde_json::json!({ "profileId": profile_id }),
            created_at: &now,
        },
    )?;

    let profile = ProfileSummary {
        id: profile_id,
        display_name: display_name.to_string(),
        role: ProfileRole::Admin,
    };
    state.set_session(UnlockedSession {
        conn,
        master_key: master_key.clone(),
        profile,
        last_activity: Instant::now(),
    });
    Ok(state.session_info())
}

#[tauri::command]
pub async fn unlock_vault(
    state: State<'_, AppState>,
    passphrase: String,
) -> AppResult<SessionInfo> {
    if !state.initialized() {
        return Err(AppError::NotInitialized);
    }
    let master_key =
        vault::unlock(&state.vault_path, &passphrase).map_err(|_| AppError::InvalidPassphrase)?;
    let mut conn = db::open_encrypted(&state.db_path, &master_key)?;
    db::apply_migrations(&mut conn)?;
    let stored = db::profiles::first(&conn)?
        .ok_or_else(|| AppError::Internal("vault is missing its local profile".to_string()))?;
    let profile = ProfileSummary {
        id: stored.id,
        display_name: stored.display_name,
        role: stored.role,
    };
    state.set_session(UnlockedSession {
        conn,
        master_key,
        profile,
        last_activity: Instant::now(),
    });
    Ok(state.session_info())
}

#[tauri::command]
pub async fn lock_vault(state: State<'_, AppState>) -> AppResult<SessionInfo> {
    state.clear_session();
    Ok(state.session_info())
}

#[tauri::command]
pub async fn touch_session(state: State<'_, AppState>) -> AppResult<SessionInfo> {
    state.with_session(|_| Ok(()))?;
    Ok(state.session_info())
}
