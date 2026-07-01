use crate::isolation::{attach_pid, build_report, configure_command, prepare_cgroup, IsolationLimits, IsolationReport};
use crate::metrics::{harvest_child_resources, ResourceMetrics};
use crate::McpError;
use serde_json::{json, Value};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::timeout;
use tracing::{info, warn};

const DEFAULT_CALL_TIMEOUT: Duration = Duration::from_secs(60);
const KILL_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
const PROTOCOL_VERSION: &str = "2024-11-05";

fn call_timeout() -> Duration {
    std::env::var("RMNG_MCP_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(DEFAULT_CALL_TIMEOUT)
}

/// Map RMNG allowlist tool ids (e.g. `git.log`) to MCP wire names (e.g. `git_log`).
pub fn wire_tool_name(allowlist_name: &str) -> String {
    allowlist_name.replace('.', "_")
}

/// Result of an isolated MCP subprocess call (Sprint 10 + Sprint 20 resources).
#[derive(Debug, Clone)]
pub struct McpCallResult {
    pub output: String,
    pub pid: Option<u32>,
    pub duration_ms: u64,
    pub resources: ResourceMetrics,
    pub isolation: IsolationReport,
}

/// Spawn an allowlisted MCP server, run initialize handshake, and call one tool.
pub async fn call_tool(
    command: &str,
    args: &[String],
    tool_name: &str,
    tool_args: &Value,
) -> Result<String, McpError> {
    call_tool_isolated(command, args, tool_name, tool_args, None)
        .await
        .map(|r| r.output)
}

/// Isolated MCP call with optional cgroup / rlimit constraints.
pub async fn call_tool_isolated(
    command: &str,
    args: &[String],
    tool_name: &str,
    tool_args: &Value,
    limits: Option<&IsolationLimits>,
) -> Result<McpCallResult, McpError> {
    let started = Instant::now();
    let limits = limits.cloned().unwrap_or_default();
    let (cgroup_path, cgroup_warnings) = if limits.is_active() {
        prepare_cgroup(&limits)
    } else {
        (None, Vec::new())
    };

    let wire_name = wire_tool_name(tool_name);
    let mut cmd = Command::new(command);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    if limits.is_active() {
        configure_command(&mut cmd, &limits);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| McpError::Spawn(format!("{command}: {e}")))?;

    let pid = child.id();
    if let (Some(ref cg), Some(pid)) = (&cgroup_path, pid) {
        if let Err(e) = attach_pid(cg, pid) {
            warn!(pid, error = %e, "cgroup attach failed");
        }
    }

    info!(
        command,
        pid = ?pid,
        tool = %tool_name,
        isolated = limits.is_active(),
        "mcp subprocess spawned"
    );

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| McpError::Protocol("missing stdin".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| McpError::Protocol("missing stdout".into()))?;

    let call_result = timeout(
        call_timeout(),
        session_call(stdin, stdout, &wire_name, tool_args),
    )
    .await;

    let (output, mut resources) = match call_result {
        Ok(Ok(output)) => {
            let resources = cleanup_child(&mut child).await;
            (output, resources)
        }
        Ok(Err(e)) => {
            let _ = cleanup_child(&mut child).await;
            return Err(e);
        }
        Err(_) => {
            let _ = cleanup_child(&mut child).await;
            return Err(McpError::Timeout(format!(
                "mcp call {tool_name} exceeded {}s",
                call_timeout().as_secs()
            )));
        }
    };

    let duration_ms = started.elapsed().as_millis() as u64;
    resources = resources.with_runtime(duration_ms);

    let mut warnings = cgroup_warnings;
    if limits.is_active() && cgroup_path.is_none() && limits.cgroup {
        warnings.push("cgroup limits not applied".into());
    }

    Ok(McpCallResult {
        output,
        pid,
        duration_ms,
        resources,
        isolation: build_report(&limits, cgroup_path, warnings),
    })
}

async fn cleanup_child(child: &mut Child) -> ResourceMetrics {
    let pid = child.id();
    let _ = child.kill().await;
    #[cfg(unix)]
    {
        if let Some(pid) = pid {
            tokio::time::sleep(Duration::from_millis(10)).await;
            return harvest_child_resources(pid);
        }
    }
    let _ = timeout(KILL_WAIT_TIMEOUT, child.wait()).await;
    ResourceMetrics::default()
}

async fn session_call(
    mut stdin: tokio::process::ChildStdin,
    stdout: tokio::process::ChildStdout,
    wire_tool: &str,
    tool_args: &Value,
) -> Result<String, McpError> {
    let mut reader = BufReader::new(stdout);

    send_request(
        &mut stdin,
        &mut reader,
        1,
        "initialize",
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": { "name": "rmng-mcp", "version": "0.1.0" }
        }),
    )
    .await?;

    send_notification(
        &mut stdin,
        "notifications/initialized",
        json!({}),
    )
    .await?;

    let call_result = send_request(
        &mut stdin,
        &mut reader,
        2,
        "tools/call",
        json!({
            "name": wire_tool,
            "arguments": tool_args
        }),
    )
    .await?;

    extract_tool_text(&call_result)
}

async fn send_notification(
    stdin: &mut tokio::process::ChildStdin,
    method: &str,
    params: Value,
) -> Result<(), McpError> {
    let msg = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    });
    write_line(stdin, &msg).await
}

async fn send_request(
    stdin: &mut tokio::process::ChildStdin,
    reader: &mut BufReader<tokio::process::ChildStdout>,
    id: u64,
    method: &str,
    params: Value,
) -> Result<Value, McpError> {
    let msg = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    });
    write_line(stdin, &msg).await?;
    read_response_for_id(reader, id).await
}

async fn write_line(stdin: &mut tokio::process::ChildStdin, msg: &Value) -> Result<(), McpError> {
    let line = serde_json::to_string(msg).map_err(|e| McpError::Protocol(e.to_string()))?;
    stdin
        .write_all(line.as_bytes())
        .await
        .map_err(McpError::Io)?;
    stdin.write_all(b"\n").await.map_err(McpError::Io)?;
    stdin.flush().await.map_err(McpError::Io)?;
    Ok(())
}

async fn read_response_for_id(
    reader: &mut BufReader<tokio::process::ChildStdout>,
    id: u64,
) -> Result<Value, McpError> {
    for _ in 0..32 {
        let line = read_line(reader).await?;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value =
            serde_json::from_str(&line).map_err(|e| McpError::Protocol(e.to_string()))?;

        if value.get("id").and_then(|v| v.as_u64()) == Some(id) {
            if let Some(err) = value.get("error") {
                return Err(McpError::Tool(
                    err.get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("mcp error")
                        .to_string(),
                ));
            }
            return value
                .get("result")
                .cloned()
                .ok_or_else(|| McpError::Protocol("response missing result".into()));
        }
    }
    Err(McpError::Protocol(format!("no response for id {id}")))
}

async fn read_line(reader: &mut BufReader<tokio::process::ChildStdout>) -> Result<String, McpError> {
    let mut line = String::new();
    reader.read_line(&mut line).await.map_err(McpError::Io)?;
    Ok(line)
}

fn extract_tool_text(result: &Value) -> Result<String, McpError> {
    if result
        .get("isError")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        let text = result
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("mcp tool returned error");
        return Err(McpError::Tool(text.to_string()));
    }

    let parts: Vec<String> = result
        .get("content")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    if parts.is_empty() {
        return Ok(result.to_string());
    }
    Ok(parts.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_dot_names_to_underscore() {
        assert_eq!(wire_tool_name("git.log"), "git_log");
        assert_eq!(wire_tool_name("get_issue"), "get_issue");
    }
}
