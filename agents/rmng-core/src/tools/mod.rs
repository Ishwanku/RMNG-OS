mod exec;
mod git;
mod github;
mod kernel;

use crate::tool::ToolResult;
use crate::RmngError;

pub async fn dispatch(name: &str, args: &serde_json::Value) -> Result<ToolResult, RmngError> {
    match name {
        "kernel.status" => kernel::status().await,
        "kernel.build" => kernel::build(args).await,
        "kernel.apply_patches" => kernel::apply_patches().await,
        "git.status" => git::status(args).await,
        "git.diff" => git::diff(args).await,
        "github.pr_status" => github::pr_status(args).await,
        other => Err(RmngError::ToolFailed(format!("no handler registered for tool: {other}"))),
    }
}

/// Tools with Rust handlers registered in this module.
pub fn registered_tools() -> &'static [&'static str] {
    &[
        "kernel.status",
        "kernel.build",
        "kernel.apply_patches",
        "git.status",
        "git.diff",
        "github.pr_status",
    ]
}