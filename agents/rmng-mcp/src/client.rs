use crate::McpError;
use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

const CALL_TIMEOUT: Duration = Duration::from_secs(60);
const PROTOCOL_VERSION: &str = "2024-11-05";

/// Map RMNG allowlist tool ids (e.g. `git.log`) to MCP wire names (e.g. `git_log`).
pub fn wire_tool_name(allowlist_name: &str) -> String {
    allowlist_name.replace('.', "_")
}

/// Spawn an allowlisted MCP server, run initialize handshake, and call one tool.
pub async fn call_tool(
    command: &str,
    args: &[String],
    tool_name: &str,
    tool_args: &Value,
) -> Result<String, McpError> {
    let wire_name = wire_tool_name(tool_name);
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| McpError::Spawn(format!("{command}: {e}")))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| McpError::Protocol("missing stdin".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| McpError::Protocol("missing stdout".into()))?;

    let result = timeout(
        CALL_TIMEOUT,
        session_call(stdin, stdout, &wire_name, tool_args),
    )
    .await
    .map_err(|_| McpError::Timeout(format!("mcp call {tool_name}")))?;

    let _ = child.kill().await;
    result
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
        // Skip notifications / unrelated messages
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