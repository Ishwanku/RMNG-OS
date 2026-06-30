use crate::tool::ToolResult;
use crate::RmngError;
use std::path::Path;
use tokio::process::Command;

/// Run a program directly — never invokes a shell.
pub async fn run_program(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<ToolResult, RmngError> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(dir) = cwd {
        if !dir.is_dir() {
            return Err(RmngError::ToolFailed(format!(
                "working directory not found: {}",
                dir.display()
            )));
        }
        cmd.current_dir(dir);
    }
    let output = cmd
        .output()
        .await
        .map_err(|e| RmngError::ToolFailed(format!("failed to spawn {program}: {e}")))?;

    Ok(ToolResult {
        success: output.status.success(),
        output: String::from_utf8_lossy(&output.stdout).to_string()
            + &String::from_utf8_lossy(&output.stderr),
        exit_code: output.status.code(),
    })
}

/// Validate a user-supplied repo path — no shell metacharacters, no traversal.
pub fn validate_repo_path(path: &str) -> Result<std::path::PathBuf, RmngError> {
    if path.is_empty() {
        return Err(RmngError::InvalidIntent("empty path".into()));
    }
    if path.contains("..") || path.contains(';') || path.contains('|') || path.contains('&') {
        return Err(RmngError::InvalidIntent(format!("unsafe path: {path}")));
    }
    let p = std::path::PathBuf::from(path);
    if !p.is_absolute() {
        return Err(RmngError::InvalidIntent("path must be absolute".into()));
    }
    Ok(p)
}
