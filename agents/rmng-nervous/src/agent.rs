use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// RMNG specialist agent manifest (`agents/definitions/*.yaml`).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AgentDefinition {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub allowed_native_tools: Vec<String>,
    #[serde(default)]
    pub allowed_mcp_tools: Vec<String>,
}

/// Loaded set of agent definitions from `agents/definitions/`.
#[derive(Debug, Clone)]
pub struct AgentRegistry {
    root: PathBuf,
    agents: HashMap<String, AgentDefinition>,
}

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("agent not found: {0}")]
    NotFound(String),
    #[error("read definitions: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse agent definition: {0}")]
    Parse(String),
}

impl AgentRegistry {
    pub fn load() -> Result<Self, AgentError> {
        Self::load_from(definitions_root())
    }

    pub fn load_from(root: impl AsRef<Path>) -> Result<Self, AgentError> {
        let root = root.as_ref().to_path_buf();
        let mut agents = HashMap::new();

        if !root.is_dir() {
            tracing::warn!(
                path = %root.display(),
                "agent definitions directory missing"
            );
            return Ok(Self { root, agents });
        }

        for entry in std::fs::read_dir(&root)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "yaml" && e != "yml") {
                continue;
            }
            let raw = std::fs::read_to_string(&path).map_err(AgentError::Io)?;
            let def: AgentDefinition = serde_yaml::from_str(&raw).map_err(|e| {
                AgentError::Parse(format!("{}: {e}", path.display()))
            })?;
            if agents.contains_key(&def.id) {
                return Err(AgentError::Parse(format!(
                    "duplicate agent id '{}' in {}",
                    def.id,
                    path.display()
                )));
            }
            agents.insert(def.id.clone(), def);
        }

        Ok(Self { root, agents })
    }

    pub fn get(&self, id: &str) -> Result<&AgentDefinition, AgentError> {
        self.agents
            .get(id)
            .ok_or_else(|| AgentError::NotFound(id.to_string()))
    }

    pub fn agent_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.agents.keys().cloned().collect();
        ids.sort();
        ids
    }

    pub fn definitions_root(&self) -> &Path {
        &self.root
    }
}

impl AgentDefinition {
    /// Whether a native tool name is permitted (supports `prefix.*` wildcards).
    pub fn allows_native_tool(&self, tool: &str) -> bool {
        pattern_matches_any(tool, &self.allowed_native_tools)
    }

    /// Whether an MCP proxy is permitted. Entries use `server:tool` or `server:*` format.
    pub fn allows_mcp_tool(&self, server: &str, tool: &str) -> bool {
        let key = format!("{server}:{tool}");
        self.allowed_mcp_tools.iter().any(|p| {
            if p == "*" {
                return true;
            }
            if let Some(prefix) = p.strip_suffix(":*") {
                return server == prefix;
            }
            p == &key || pattern_matches(&key, p)
        })
    }

    pub fn allows_core_intent(&self, intent: &rmng_core::CoreIntent) -> Result<(), String> {
        use rmng_core::CoreIntent;
        match intent {
            CoreIntent::PlanOnly { .. } => Ok(()),
            CoreIntent::ToolExecute { target, .. } => {
                if self.allows_native_tool(target) {
                    Ok(())
                } else {
                    Err(format!(
                        "agent '{}' cannot execute native tool '{target}'",
                        self.id
                    ))
                }
            }
            CoreIntent::McpProxy {
                mcp_server,
                mcp_tool,
                ..
            } => {
                if self.allows_mcp_tool(mcp_server, mcp_tool) {
                    Ok(())
                } else {
                    Err(format!(
                        "agent '{}' cannot proxy MCP {mcp_server}.{mcp_tool}",
                        self.id
                    ))
                }
            }
        }
    }
}

fn pattern_matches_any(value: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| pattern_matches(value, p))
}

fn pattern_matches(value: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix(".*") {
        return value == prefix || value.starts_with(&format!("{prefix}."));
    }
    if let Some(prefix) = pattern.strip_suffix(":*") {
        return value.starts_with(&format!("{prefix}:"));
    }
    value == pattern
}

pub fn definitions_root() -> PathBuf {
    if let Ok(root) = std::env::var("RMNG_PROJECT_ROOT") {
        let path = PathBuf::from(root).join("agents/definitions");
        if path.is_dir() {
            return path;
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let path = PathBuf::from(home).join("dev/projects/RMNG-OS/agents/definitions");
        if path.is_dir() {
            return path;
        }
    }
    PathBuf::from("agents/definitions")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_registry() -> AgentRegistry {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../definitions");
        AgentRegistry::load_from(root).expect("fixture agents")
    }

    #[test]
    fn loads_kernel_and_repo_agents() {
        let reg = fixture_registry();
        assert!(reg.get("kernel-engineer").is_ok());
        assert!(reg.get("repo-keeper").is_ok());
    }

    #[test]
    fn kernel_engineer_denies_git_tools() {
        let reg = fixture_registry();
        let agent = reg.get("kernel-engineer").unwrap();
        assert!(agent.allows_native_tool("kernel.status"));
        assert!(!agent.allows_native_tool("git.status"));
    }

    #[test]
    fn repo_keeper_allows_git_not_kernel() {
        let reg = fixture_registry();
        let agent = reg.get("repo-keeper").unwrap();
        assert!(agent.allows_native_tool("git.status"));
        assert!(agent.allows_native_tool("git.diff"));
        assert!(!agent.allows_native_tool("kernel.build"));
    }

    #[test]
    fn wildcard_prefix_matches() {
        assert!(pattern_matches("kernel.status", "kernel.*"));
        assert!(!pattern_matches("git.status", "kernel.*"));
    }
}