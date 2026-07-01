use super::exec::{run_program, validate_repo_path};
use crate::tool::ToolResult;
use crate::RmngError;
use std::path::PathBuf;

pub(crate) fn default_repo() -> PathBuf {
    if let Ok(p) = std::env::var("RMNG_PROJECT_ROOT") {
        return PathBuf::from(p);
    }
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(&home).join("dev/projects/RMNG-OS");
        if p.exists() {
            return p;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub(crate) fn resolve_repo(args: &serde_json::Value) -> Result<PathBuf, RmngError> {
    match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => validate_repo_path(p),
        None => Ok(default_repo()),
    }
}

pub async fn status(args: &serde_json::Value) -> Result<ToolResult, RmngError> {
    let repo = resolve_repo(args)?;
    if !repo.join(".git").exists() {
        return Err(RmngError::ToolFailed(format!(
            "not a git repository: {}",
            repo.display()
        )));
    }
    run_program("git", &["status", "--porcelain", "-b"], Some(&repo)).await
}

pub async fn diff(args: &serde_json::Value) -> Result<ToolResult, RmngError> {
    let repo = resolve_repo(args)?;
    if !repo.join(".git").exists() {
        return Err(RmngError::ToolFailed(format!(
            "not a git repository: {}",
            repo.display()
        )));
    }
    let staged = args.get("staged").and_then(|v| v.as_bool()).unwrap_or(false);
    if staged {
        run_program("git", &["diff", "--stat", "--cached"], Some(&repo)).await
    } else {
        run_program("git", &["diff", "--stat"], Some(&repo)).await
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_path() {
        assert!(validate_repo_path("/tmp/../etc/passwd").is_err());
        assert!(validate_repo_path("relative/path").is_err());
    }
}
