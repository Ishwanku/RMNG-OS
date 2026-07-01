use super::git::{default_repo, resolve_repo};
use super::exec::run_program;
use crate::tool::ToolResult;
use crate::RmngError;

pub async fn pr_status(args: &serde_json::Value) -> Result<ToolResult, RmngError> {
    let repo = resolve_repo(args).unwrap_or_else(|_| default_repo());
    if !repo.join(".git").exists() {
        return Err(RmngError::ToolFailed(format!(
            "not a git repository: {}",
            repo.display()
        )));
    }
    run_program(
        "gh",
        &[
            "pr",
            "status",
            "--json",
            "title,state,url,number",
        ],
        Some(&repo),
    )
    .await
}