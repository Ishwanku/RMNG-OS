use crate::tool::ToolResult;
use crate::RmngError;
use std::path::PathBuf;
use tokio::process::Command;

fn project_root() -> PathBuf {
    if let Ok(p) = std::env::var("RMNG_PROJECT_ROOT") {
        return PathBuf::from(p);
    }
    if let Ok(home) = std::env::var("HOME") {
        let default = PathBuf::from(&home).join("dev/projects/RMNG-OS");
        if default.exists() {
            return default;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn status_script() -> PathBuf {
    if let Ok(p) = std::env::var("RMNG_STATUS_SCRIPT") {
        return PathBuf::from(p);
    }
    if let Ok(home) = std::env::var("HOME") {
        let symlink = PathBuf::from(&home).join("scripts/rmng-status.sh");
        if symlink.exists() {
            return symlink;
        }
    }
    project_root().join("scripts/status.sh")
}

async fn run_script(script: PathBuf, args: &[&str]) -> Result<ToolResult, RmngError> {
    if !script.exists() {
        return Err(RmngError::ToolFailed(format!("script not found: {}", script.display())));
    }
    let output = Command::new("bash")
        .arg(&script)
        .args(args)
        .output()
        .await
        .map_err(|e| RmngError::ToolFailed(e.to_string()))?;

    Ok(ToolResult {
        success: output.status.success(),
        output: String::from_utf8_lossy(&output.stdout).to_string()
            + &String::from_utf8_lossy(&output.stderr),
        exit_code: output.status.code(),
    })
}

pub async fn status() -> Result<ToolResult, RmngError> {
    run_script(status_script(), &[]).await
}

pub async fn build(args: &serde_json::Value) -> Result<ToolResult, RmngError> {
    let target = args.get("target").and_then(|v| v.as_str()).unwrap_or("all");
    let script = project_root().join("scripts/build.sh");
    run_script(script, &[target]).await
}

pub async fn apply_patches() -> Result<ToolResult, RmngError> {
    let script = project_root().join("scripts/rebuild-with-patches.sh");
    run_script(script, &[]).await
}
