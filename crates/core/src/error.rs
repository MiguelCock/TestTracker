use serde::ser::{Serialize, Serializer};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("vault is not initialized")]
    NotInitialized,
    #[error("vault is locked")]
    Locked,
    #[error("invalid passphrase")]
    InvalidPassphrase,
    #[error("session expired")]
    SessionExpired,
    #[error("not found")]
    NotFound,
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("permission denied")]
    PermissionDenied,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("cryptographic operation failed")]
    Crypto,
    #[error("database operation failed")]
    Database(#[from] rusqlite::Error),
    #[error("filesystem operation failed")]
    Io(#[from] std::io::Error),
    #[error("serialization failed")]
    Serde(#[from] serde_json::Error),
    #[error("external service error: {0}")]
    External(String),
    #[error("plugin error: {0}")]
    Plugin(String),
    #[error("internal error")]
    Internal(String),
}

impl AppError {
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::NotInitialized => "The vault has not been initialized.",
            Self::Locked => "The vault is locked.",
            Self::InvalidPassphrase => "Invalid passphrase.",
            Self::SessionExpired => "The session expired. Unlock the vault to continue.",
            Self::NotFound => "The requested record was not found.",
            Self::Validation(_) => "The submitted data is invalid.",
            Self::PermissionDenied => "You do not have permission to perform this action.",
            Self::Conflict(_) => "The operation conflicts with existing data.",
            Self::Crypto => "A cryptographic operation failed.",
            Self::Database(_) => "A database operation failed.",
            Self::Io(_) => "A filesystem operation failed.",
            Self::Serde(_) => "A serialization operation failed.",
            Self::External(_) => "An external service error occurred.",
            Self::Plugin(_) => "The plugin failed.",
            Self::Internal(_) => "An internal error occurred.",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Validation(message)
            | Self::Conflict(message)
            | Self::External(message)
            | Self::Plugin(message) => serializer.serialize_str(message),
            _ => serializer.serialize_str(self.user_message()),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
