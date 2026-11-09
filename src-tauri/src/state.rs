use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rusqlite::Connection;
use zeroize::Zeroizing;

use crate::ai::agent::{AgentPreviewRecord, AgentProposalRecord};
use crate::ai::assistant::{AssistantPreviewRecord, AssistantRunRecord};
use crate::ai::model::AiPreview;
use crate::domain::models::{ProfileSummary, SessionInfo};
use crate::error::{AppError, AppResult};
use crate::imports::model::{ImportDraftRecord, ImportExtractionRecord, ImportPreviewRecord};
use crate::mcp::runtime::McpRuntime;
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
    agent_previews: Mutex<HashMap<String, AgentPreviewRecord>>,
    agent_proposals: Mutex<HashMap<String, AgentProposalRecord>>,
    assistant_previews: Mutex<HashMap<String, AssistantPreviewRecord>>,
    assistant_runs: Mutex<HashMap<String, AssistantRunRecord>>,
    import_extractions: Mutex<HashMap<String, ImportExtractionRecord>>,
    import_previews: Mutex<HashMap<String, ImportPreviewRecord>>,
    import_drafts: Mutex<HashMap<String, ImportDraftRecord>>,
    pub mcp: McpRuntime,
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
            agent_previews: Mutex::new(HashMap::new()),
            agent_proposals: Mutex::new(HashMap::new()),
            assistant_previews: Mutex::new(HashMap::new()),
            assistant_runs: Mutex::new(HashMap::new()),
            import_extractions: Mutex::new(HashMap::new()),
            import_previews: Mutex::new(HashMap::new()),
            import_drafts: Mutex::new(HashMap::new()),
            mcp: McpRuntime::new(),
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
            self.mcp.stop();
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
        self.mcp.stop();
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
        let mut agent_previews = self
            .agent_previews
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        agent_previews.clear();
        let mut agent_proposals = self
            .agent_proposals
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        agent_proposals.clear();
        let mut assistant_previews = self
            .assistant_previews
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        assistant_previews.clear();
        let mut assistant_runs = self
            .assistant_runs
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        assistant_runs.clear();
        let mut import_extractions = self
            .import_extractions
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        import_extractions.clear();
        let mut import_previews = self
            .import_previews
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        import_previews.clear();
        let mut import_drafts = self.import_drafts.lock().unwrap_or_else(|e| e.into_inner());
        import_drafts.clear();
    }

    pub fn insert_agent_preview(&self, preview: AgentPreviewRecord) {
        let mut previews = self
            .agent_previews
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        previews.retain(|_, preview| !preview.is_expired());
        previews.insert(preview.id.clone(), preview);
    }

    pub fn take_agent_preview(&self, id: &str) -> Option<AgentPreviewRecord> {
        let mut previews = self
            .agent_previews
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        previews.retain(|_, preview| !preview.is_expired());
        previews.remove(id)
    }

    pub fn insert_agent_proposal(&self, proposal: AgentProposalRecord) {
        let mut proposals = self
            .agent_proposals
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        proposals.retain(|_, proposal| !proposal.is_expired());
        proposals.insert(proposal.id.clone(), proposal);
    }

    pub fn take_agent_proposal(&self, id: &str) -> Option<AgentProposalRecord> {
        let mut proposals = self
            .agent_proposals
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        proposals.retain(|_, proposal| !proposal.is_expired());
        proposals.remove(id)
    }

    pub fn insert_import_extraction(&self, extraction: ImportExtractionRecord) {
        let mut extractions = self
            .import_extractions
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        extractions.retain(|_, extraction| !extraction.is_expired());
        extractions.insert(extraction.id.clone(), extraction);
    }

    pub fn get_import_extraction(&self, id: &str) -> Option<ImportExtractionRecord> {
        let mut extractions = self
            .import_extractions
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        extractions.retain(|_, extraction| !extraction.is_expired());
        extractions.get(id).cloned()
    }

    pub fn insert_import_preview(&self, preview: ImportPreviewRecord) {
        let mut previews = self
            .import_previews
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        previews.retain(|_, preview| !preview.is_expired());
        previews.insert(preview.id.clone(), preview);
    }

    pub fn take_import_preview(&self, id: &str) -> Option<ImportPreviewRecord> {
        let mut previews = self
            .import_previews
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        previews.retain(|_, preview| !preview.is_expired());
        previews.remove(id)
    }

    pub fn insert_import_draft(&self, draft: ImportDraftRecord) {
        let mut drafts = self.import_drafts.lock().unwrap_or_else(|e| e.into_inner());
        drafts.retain(|_, draft| !draft.is_expired());
        drafts.insert(draft.id.clone(), draft);
    }

    pub fn take_import_draft(&self, id: &str) -> Option<ImportDraftRecord> {
        let mut drafts = self.import_drafts.lock().unwrap_or_else(|e| e.into_inner());
        drafts.retain(|_, draft| !draft.is_expired());
        drafts.remove(id)
    }

    pub fn insert_assistant_preview(&self, preview: AssistantPreviewRecord) {
        let mut previews = self
            .assistant_previews
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        previews.retain(|_, preview| !preview.is_expired());
        previews.insert(preview.id.clone(), preview);
    }

    pub fn take_assistant_preview(&self, id: &str) -> Option<AssistantPreviewRecord> {
        let mut previews = self
            .assistant_previews
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        previews.retain(|_, preview| !preview.is_expired());
        previews.remove(id)
    }

    pub fn insert_assistant_run(&self, run: AssistantRunRecord) {
        let mut runs = self
            .assistant_runs
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        runs.retain(|_, run| !run.is_expired());
        runs.insert(run.id.clone(), run);
    }

    pub fn take_assistant_run(&self, id: &str) -> Option<AssistantRunRecord> {
        let mut runs = self
            .assistant_runs
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        runs.retain(|_, run| !run.is_expired());
        runs.remove(id)
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
            self.mcp.stop();
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

    pub fn with_session_no_touch<T>(
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
            self.mcp.stop();
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
        action(session)
    }
}
