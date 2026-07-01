use crate::layer::{AgentLayer, LayerAgent};
use rmng_core::{AgentLlmOverride, CoreIntent, LlmProviderKind};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// RMNG specialist agent manifest (`agents/definitions/*.yaml`).
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AgentDefinition {
    pub id: String,
    pub description: String,
    /// Multi-level layer (ADR-017). Defaults to L3 for backward compatibility.
    #[serde(default = "default_layer")]
    pub layer: AgentLayer,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub allowed_native_tools: Vec<String>,
    #[serde(default)]
    pub allowed_mcp_tools: Vec<String>,
    /// Explicit handoff targets (agent ids). Wildcards not supported here.
    #[serde(default)]
    pub delegates_to: Vec<String>,
    /// Named LLM profile from `~/.rmng/config.toml` `[[profiles]]` (Sprint 7).
    #[serde(default)]
    pub llm_profile: Option<String>,
    /// Per-agent provider override (used when `llm_profile` is unset).
    #[serde(default)]
    pub llm_provider: Option<LlmProviderKind>,
    /// Per-agent model id override.
    #[serde(default)]
    pub model: Option<String>,
    /// Per-agent daily USD cap for budget enforcement (Sprint 12).
    #[serde(default)]
    pub daily_budget_usd: Option<f64>,
    /// Fallback profile names from `~/.rmng/config.toml` (Sprint 8).
    #[serde(default)]
    pub llm_fallback: Vec<String>,
}

fn default_layer() -> AgentLayer {
    AgentLayer::L3
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
    #[error("handoff not permitted: {0}")]
    HandoffDenied(String),
}

impl AgentRegistry {
    pub fn load() -> Result<Self, AgentError> {
        Self::load_from(definitions_root())
    }

    pub fn load_from(root: impl AsRef<Path>) -> Result<Self, AgentError> {
        let root = root.as_ref().to_path_buf();
        let mut agents = HashMap::new();

        if !root.is_dir() {
            tracing::warn!(path = %root.display(), "agent definitions directory missing");
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

    pub fn by_layer(&self, layer: AgentLayer) -> Vec<&AgentDefinition> {
        let mut out: Vec<&AgentDefinition> = self
            .agents
            .values()
            .filter(|a| a.layer == layer)
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    /// Find the best delegate agent for a native tool (lowest layer that allows it).
    pub fn find_delegate_for_tool(&self, tool: &str) -> Option<&AgentDefinition> {
        let mut candidates: Vec<&AgentDefinition> = self
            .agents
            .values()
            .filter(|a| a.allows_native_tool(tool))
            .collect();
        candidates.sort_by_key(|a| std::cmp::Reverse(a.layer.numeric()));
        candidates.first().copied()
    }

    /// Resolve explicit or implicit handoff target for an orchestrator.
    pub fn resolve_handoff_target(
        &self,
        from: &AgentDefinition,
        hint: &str,
    ) -> Result<&AgentDefinition, AgentError> {
        if let Ok(agent) = self.get(hint) {
            from.validate_handoff_to(agent)?;
            return Ok(agent);
        }
        if hint.contains('.') {
            if let Some(agent) = self.find_delegate_for_tool(hint) {
                from.validate_handoff_to(agent)?;
                return Ok(agent);
            }
        }
        for id in &from.delegates_to {
            if let Ok(agent) = self.get(id) {
                if hint.is_empty() || agent.id.contains(hint) || hint.contains(&agent.id) {
                    from.validate_handoff_to(agent)?;
                    return Ok(agent);
                }
            }
        }
        Err(AgentError::HandoffDenied(format!(
            "no delegate from '{}' for hint '{}'",
            from.id, hint
        )))
    }

    pub fn definitions_root(&self) -> &Path {
        &self.root
    }
}

impl AgentLlmOverride for AgentDefinition {
    fn llm_profile_name(&self) -> Option<&str> {
        self.llm_profile.as_deref()
    }

    fn llm_provider_override(&self) -> Option<LlmProviderKind> {
        self.llm_provider
    }

    fn model_override(&self) -> Option<&str> {
        self.model.as_deref()
    }

    fn llm_fallback_profiles(&self) -> &[String] {
        &self.llm_fallback
    }
}

impl LayerAgent for AgentDefinition {
    fn layer(&self) -> AgentLayer {
        self.layer
    }

    fn can_handoff_to(&self, target: &dyn LayerAgent) -> bool {
        self.layer.can_delegate_to(target.layer())
    }
}

impl AgentDefinition {
    pub fn allows_native_tool(&self, tool: &str) -> bool {
        pattern_matches_any(tool, &self.allowed_native_tools)
    }

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

    pub fn allows_core_intent(&self, intent: &CoreIntent) -> Result<(), String> {
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

    pub fn validate_handoff_to(&self, target: &AgentDefinition) -> Result<(), AgentError> {
        if !self.can_handoff_to(target) {
            return Err(AgentError::HandoffDenied(format!(
                "{} ({}) cannot hand off to {} ({})",
                self.id,
                self.layer,
                target.id,
                target.layer
            )));
        }
        if !self.delegates_to.is_empty() && !self.delegates_to.iter().any(|d| d == &target.id) {
            return Err(AgentError::HandoffDenied(format!(
                "{} is not listed in {} delegates_to",
                target.id, self.id
            )));
        }
        Ok(())
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
    fn loads_all_layer_agents() {
        let reg = fixture_registry();
        assert!(reg.get("kernel-engineer").is_ok());
        assert!(reg.get("repo-keeper").is_ok());
        assert!(reg.get("system-health").is_ok());
        assert!(reg.get("swarm-coordinator").is_ok());
    }

    #[test]
    fn kernel_engineer_is_l1() {
        let reg = fixture_registry();
        let agent = reg.get("kernel-engineer").unwrap();
        assert_eq!(agent.layer, AgentLayer::L1);
        assert!(agent.allows_native_tool("kernel.status"));
        assert!(!agent.allows_native_tool("git.status"));
    }

    #[test]
    fn swarm_coordinator_delegates_to_repo_keeper() {
        let reg = fixture_registry();
        let orch = reg.get("swarm-coordinator").unwrap();
        let repo = reg.get("repo-keeper").unwrap();
        assert!(orch.validate_handoff_to(repo).is_ok());
        assert!(orch.validate_handoff_to(reg.get("kernel-engineer").unwrap()).is_ok());
    }

    #[test]
    fn find_delegate_for_git_status() {
        let reg = fixture_registry();
        let delegate = reg.find_delegate_for_tool("git.status").unwrap();
        assert_eq!(delegate.id, "repo-keeper");
    }


    #[test]
    fn l3_testing_agents_include_testing_skills() {
        let reg = fixture_registry();
        let testing_skills = [
            "run-tests",
            "validate-output",
            "test-coverage-check",
            "regression-check",
        ];
        for id in ["repo-keeper", "research-curator"] {
            let agent = reg.get(id).expect(id);
            for skill in testing_skills {
                assert!(
                    agent.skills.iter().any(|s| s == skill),
                    "{id} missing skill {skill}"
                );
            }
        }
    }

    #[test]
    fn l3_sandbox_agents_include_code_execution_skill() {
        let reg = fixture_registry();
        for id in ["repo-keeper", "research-curator"] {
            let agent = reg.get(id).expect(id);
            assert!(
                agent.skills.iter().any(|s| s == "code-execution"),
                "{id} missing code-execution skill"
            );
            let intent = CoreIntent::McpProxy {
                mcp_server: "e2b".into(),
                mcp_tool: "run_code".into(),
                mcp_args: serde_json::json!({"code": "print(1)"}),
                metadata: None,
            };
            assert!(agent.allows_core_intent(&intent).is_ok(), "{id} should allow e2b:run_code");
        }
    }

    #[test]
    fn l3_agents_include_evaluation_skills() {
        let reg = fixture_registry();
        let eval_skills = ["self-critique", "output-validation", "improvement-loop"];
        for id in [
            "research-curator",
            "web-researcher",
            "repo-keeper",
            "browser-researcher",
        ] {
            let agent = reg.get(id).expect(id);
            for skill in eval_skills {
                assert!(
                    agent.skills.iter().any(|s| s == skill),
                    "{id} missing skill {skill}"
                );
            }
        }
    }
}
