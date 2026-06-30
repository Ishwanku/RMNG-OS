use crate::RmngError;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// MCP server entry from `~/.rmng/mcp-allowlist.toml`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct McpServerConfig {
    pub enabled: bool,
    pub command: String,
    pub args: Vec<String>,
    pub allowed_tools: Vec<String>,
}

/// Top-level allowlist: only explicitly configured servers may be proxied.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
pub struct McpAllowlist {
    #[serde(default)]
    pub servers: HashMap<String, McpServerConfig>,
}

impl McpAllowlist {
    pub fn config_path() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".rmng/mcp-allowlist.toml");
        }
        PathBuf::from("/tmp/rmng/mcp-allowlist.toml")
    }

    pub fn load() -> Result<Self, RmngError> {
        Self::load_from(&Self::config_path())
    }

    pub fn load_from(path: impl AsRef<Path>) -> Result<Self, RmngError> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)
            .map_err(|e| RmngError::InvalidIntent(format!("read allowlist: {e}")))?;
        toml::from_str(&raw)
            .map_err(|e| RmngError::InvalidIntent(format!("parse allowlist: {e}")))
    }

    pub fn server_config(&self, name: &str) -> Option<&McpServerConfig> {
        self.servers.get(name).filter(|s| s.enabled)
    }

    pub fn is_tool_allowed(&self, server: &str, tool: &str) -> bool {
        self.servers
            .get(server)
            .filter(|s| s.enabled)
            .map(|s| s.allowed_tools.iter().any(|t| t == tool))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_example_structure() {
        let raw = r#"
[servers.github]
enabled = true
command = "npx"
args = ["@github/github-mcp-server"]
allowed_tools = ["get_issue", "create_issue"]

[servers.git]
enabled = true
command = "uvx"
args = ["mcp-server-git"]
allowed_tools = ["git.log"]
"#;
        let list: McpAllowlist = toml::from_str(raw).unwrap();
        assert!(list.is_tool_allowed("git", "git.log"));
        assert!(!list.is_tool_allowed("git", "git.commit"));
        assert!(!list.is_tool_allowed("unknown", "git.log"));
    }
}