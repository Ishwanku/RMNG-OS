use thiserror::Error;

#[derive(Debug, Error)]
pub enum RmngError {
    #[error("invalid intent: {0}")]
    InvalidIntent(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("tool execution failed: {0}")]
    ToolFailed(String),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}
