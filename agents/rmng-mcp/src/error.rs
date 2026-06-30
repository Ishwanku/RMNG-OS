use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("spawn failed: {0}")]
    Spawn(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("tool error: {0}")]
    Tool(String),
    #[error("timeout: {0}")]
    Timeout(String),
}