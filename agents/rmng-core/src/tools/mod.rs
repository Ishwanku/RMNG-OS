mod exec;
mod git;
mod kernel;

use crate::tool::ToolResult;
use crate::RmngError;

pub async fn dispatch(name: &str, args: &serde_json::Value) -> Result<ToolResult, RmngError> {
    match name {
        "kernel.status" => kernel::status().await,
        "kernel.build" => kernel::build(args).await,
        "kernel.apply_patches" => kernel::apply_patches().await,
        "git.status" => git::status(args).await,
        other => Err(RmngError::ToolFailed(format!("unknown tool: {other}"))),
    }
}

pub fn list() -> &'static [&'static str] {
    &[
        "kernel.status",
        "kernel.build",
        "kernel.apply_patches",
        "git.status",
    ]
}
