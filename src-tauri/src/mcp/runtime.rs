use std::sync::Mutex;
use std::time::Duration;

use tauri::AppHandle;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::error::{AppError, AppResult};

use super::approvals::McpApprovalBook;
use super::auth::McpAuth;
use super::model::{McpApprovalRecord, McpApprovalResult, McpApprovalStatus, McpStatus};
use super::server;

pub const DEFAULT_PORT: u16 = 37621;

struct RunningServer {
    port: u16,
    endpoint: String,
    started_at: String,
    cancellation: CancellationToken,
    handle: tauri::async_runtime::JoinHandle<()>,
}

#[derive(Default)]
struct RuntimeState {
    running: Option<RunningServer>,
    last_port: u16,
}

pub struct McpRuntime {
    state: Mutex<RuntimeState>,
    approvals: McpApprovalBook,
}

impl McpRuntime {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(RuntimeState {
                running: None,
                last_port: DEFAULT_PORT,
            }),
            approvals: McpApprovalBook::default(),
        }
    }

    pub fn status(&self, has_token: bool) -> McpStatus {
        let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        let running = state.running.as_ref();
        McpStatus {
            running: running.is_some(),
            endpoint: running.map(|server| server.endpoint.clone()),
            port: running.map(|server| server.port).unwrap_or(state.last_port),
            has_token,
            pending_count: self.approvals.pending_count(),
            started_at: running.map(|server| server.started_at.clone()),
        }
    }

    pub fn is_running(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .running
            .is_some()
    }

    pub async fn start(&self, app: AppHandle, port: u16, token: &str) -> AppResult<u16> {
        {
            let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            if state.running.is_some() {
                return Err(AppError::Conflict(
                    "The MCP bridge is already running.".into(),
                ));
            }
            state.last_port = port;
        }
        let cancellation = CancellationToken::new();
        let auth = std::sync::Arc::new(McpAuth::new(token));
        let (ready_tx, ready_rx) = oneshot::channel();
        let task_cancellation = cancellation.clone();
        let task_app = app.clone();
        let handle = tauri::async_runtime::spawn(async move {
            let _ = server::serve(task_app, port, auth, task_cancellation, ready_tx).await;
        });
        let actual_port = match tokio::time::timeout(Duration::from_secs(5), ready_rx).await {
            Ok(Ok(Ok(port))) => port,
            Ok(Ok(Err(message))) => {
                handle.abort();
                return Err(AppError::Validation(format!(
                    "Unable to start the MCP bridge: {message}"
                )));
            }
            Ok(Err(_)) | Err(_) => {
                handle.abort();
                return Err(AppError::Validation(
                    "The MCP bridge did not start in time.".into(),
                ));
            }
        };
        let started_at = crate::db::now();
        let endpoint = format!("http://127.0.0.1:{actual_port}/mcp");
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.last_port = actual_port;
        state.running = Some(RunningServer {
            port: actual_port,
            endpoint,
            started_at,
            cancellation,
            handle,
        });
        Ok(actual_port)
    }

    pub fn stop(&self) {
        let running = {
            let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            state.running.take()
        };
        if let Some(running) = running {
            running.cancellation.cancel();
            running.handle.abort();
        }
        self.approvals.clear();
    }

    pub fn insert_approval(&self, record: McpApprovalRecord) -> bool {
        self.approvals.insert(record)
    }

    pub fn can_accept_approval(&self, engagement_id: &str) -> bool {
        self.approvals.has_capacity(engagement_id)
    }

    pub fn pending_approvals(&self) -> Vec<McpApprovalRecord> {
        self.approvals.list_pending()
    }

    pub fn approval_status(&self, id: &str) -> Option<McpApprovalStatus> {
        self.approvals.get(id)
    }

    pub fn take_pending_approval(&self, id: &str) -> Option<McpApprovalRecord> {
        self.approvals.take_pending(id)
    }

    pub fn complete_approval(&self, record: &McpApprovalRecord, result: McpApprovalResult) {
        self.approvals.complete(record, result);
    }

    pub fn reject_approval(&self, record: &McpApprovalRecord) {
        self.approvals.reject(record);
    }

    pub fn remove_approval(&self, id: &str) {
        self.approvals.remove(id);
    }

    pub fn restore_approval(&self, record: McpApprovalRecord) {
        self.approvals.restore(record);
    }
}
