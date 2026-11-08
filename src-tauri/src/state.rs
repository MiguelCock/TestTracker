use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rusqlite::Connection;
use zeroize::Zeroizing;

use crate::ai::model::AiPreview;
use crate::domain::models::{ProfileSummary, SessionInfo};
use crate::error::{AppError, AppResult};
use crate::security::crypto::KEY_LEN;
use crate::security::vault;

pub const AUTO_LOCK_SECONDS: u64 = 15 * 60;

pub struct UnlockedSession {
    pub conn: Connection,
    #[allow(dead_code)]
    pub master_key: Zeroizing<[u8; KEY_LEN]>,
    pub profile: ProfileSummary,
    pub last_activity: Instant,
}

pub struct AppState {
    pub data_dir: PathBuf,
    pub vault_path: PathBuf,
    pub db_path: PathBuf,
    session: Mutex<Option<UnlockedSession>>,
    ai_previews: Mutex<HashMap<String, AiPreview>>,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        let vault_path = data_dir.join("vault.json");
        let db_path = data_dir.join("testtracker.db");
        Self {
            data_dir,
            vault_path,
            db_path,
            session: Mutex::new(None),
            ai_previews: Mutex::new(HashMap::new()),
        }
    }

    pub fn initialized(&self) -> bool {
        vault::is_initialized(&self.vault_path)
    }

    pub fn session_info(&self) -> SessionInfo {
        let mut guard = self.session.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(session) = guard.as_ref()
            && session.last_activity.elapsed() > Duration::from_secs(AUTO_LOCK_SECONDS)
        {
            *guard = None;
            self.clear_ai_previews();
        }
        let profile = guard.as_ref().map(|session| session.profile.clone());
        SessionInfo {
            initialized: self.initialized(),
            unlocked: profile.is_some(),
            profile,
            auto_lock_seconds: AUTO_LOCK_SECONDS,
        }
    }

    pub fn set_session(&self, session: UnlockedSession) {
        let mut guard = self.session.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(session);
    }

    pub fn clear_session(&self) {
        let mut guard = self.session.lock().unwrap_or_else(|e| e.into_inner());
        *guard = None;
        drop(guard);
        self.clear_ai_previews();
    }

    pub fn insert_ai_preview(&self, preview: AiPreview) {
        let mut previews = self.ai_previews.lock().unwrap_or_else(|e| e.into_inner());
        previews.retain(|_, preview| !preview.is_expired());
        previews.insert(preview.id.clone(), preview);
    }

    pub fn take_ai_preview(&self, id: &str) -> Option<AiPreview> {
        let mut previews = self.ai_previews.lock().unwrap_or_else(|e| e.into_inner());
        previews.retain(|_, preview| !preview.is_expired());
        previews.remove(id)
    }

    pub fn clear_ai_previews(&self) {
        let mut previews = self.ai_previews.lock().unwrap_or_else(|e| e.into_inner());
        previews.clear();
    }

    pub fn with_session<T>(
        &self,
        action: impl FnOnce(&mut UnlockedSession) -> AppResult<T>,
    ) -> AppResult<T> {
        let mut guard = self.session.lock().unwrap_or_else(|e| e.into_inner());
        let expired = guard
            .as_ref()
            .map(|session| session.last_activity.elapsed() > Duration::from_secs(AUTO_LOCK_SECONDS))
            .unwrap_or(false);
        if expired {
            *guard = None;
            self.clear_ai_previews();
        }
        let session = guard.as_mut().ok_or_else(|| {
            if expired {
                AppError::SessionExpired
            } else if self.initialized() {
                AppError::Locked
            } else {
                AppError::NotInitialized
            }
        })?;
        session.last_activity = Instant::now();
        action(session)
    }
}
