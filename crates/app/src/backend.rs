use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use testtracker_core::error::{AppError, AppResult};
use testtracker_core::state::AppState;
use tokio::sync::oneshot;

pub struct Backend {
    pub state: Arc<AppState>,
    pub runtime: tokio::runtime::Runtime,
    pub data_dir: PathBuf,
}

pub fn default_data_dir() -> PathBuf {
    if let Ok(path) = std::env::var("TESTTRACKER_DATA_DIR")
        && !path.trim().is_empty()
    {
        return PathBuf::from(path);
    }
    #[cfg(target_os = "linux")]
    {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|home| home.join(".local/share")))
            .unwrap_or_else(|| PathBuf::from("."));
        return base.join("com.testtracker.desktop");
    }
    #[cfg(target_os = "macos")]
    {
        return dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("com.testtracker.desktop");
    }
    #[cfg(target_os = "windows")]
    {
        return dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("com.testtracker.desktop");
    }
    #[allow(unreachable_code)]
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.testtracker.desktop")
}

impl Backend {
    pub fn new() -> Result<Arc<Self>, String> {
        let data_dir = default_data_dir();
        std::fs::create_dir_all(&data_dir).map_err(|error| error.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&data_dir, std::fs::Permissions::from_mode(0o700));
        }
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("testtracker-worker")
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Arc::new(Self {
            state: Arc::new(AppState::new(data_dir.clone())),
            runtime,
            data_dir,
        }))
    }

    /// Run an async core service on the tokio runtime and deliver the result
    /// back to the GPUI main thread through a oneshot channel.
    pub fn call<T, F, Fut>(&self, action: F) -> oneshot::Receiver<AppResult<T>>
    where
        T: Send + 'static,
        F: FnOnce(Arc<AppState>) -> Fut + Send + 'static,
        Fut: Future<Output = AppResult<T>> + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let state = self.state.clone();
        self.runtime.spawn(async move {
            let result = action(state).await;
            let _ = tx.send(result);
        });
        rx
    }

    pub fn error_text(error: &AppError) -> String {
        match error {
            AppError::Validation(message)
            | AppError::Conflict(message)
            | AppError::External(message)
            | AppError::Plugin(message) => message.clone(),
            AppError::Internal(_) => "An internal error occurred.".into(),
            AppError::Locked | AppError::SessionExpired => {
                "The vault is locked. Unlock it to continue.".into()
            }
            AppError::NotInitialized => "The vault has not been initialized.".into(),
            AppError::InvalidPassphrase => "Invalid passphrase.".into(),
            AppError::PermissionDenied => {
                "You do not have permission to perform this action.".into()
            }
            AppError::NotFound => "The requested record was not found.".into(),
            AppError::Crypto => "A cryptographic operation failed.".into(),
            AppError::Database(_) => "A database operation failed.".into(),
            AppError::Io(_) => "A filesystem operation failed.".into(),
            AppError::Serde(_) => "A serialization operation failed.".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_errors_keep_their_user_message() {
        let error = AppError::Validation("Name is required.".into());
        assert_eq!(Backend::error_text(&error), "Name is required.");
    }

    #[test]
    fn locked_errors_are_mapped_to_a_safe_message() {
        assert_eq!(
            Backend::error_text(&AppError::Locked),
            "The vault is locked. Unlock it to continue."
        );
    }
}
