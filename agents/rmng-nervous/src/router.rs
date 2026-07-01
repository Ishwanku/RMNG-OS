use crate::agent::{AgentDefinition, AgentError, AgentRegistry};
use crate::skill::{load_skills_for_agent, AgentSkill, SkillError};
use crate::{ConnectorError, NervousConnector};
use rmng_core::CoreIntent;

/// Resolved routing context for an agent invocation.
#[derive(Debug, Clone)]
pub struct AgentRoute {
    pub agent: AgentDefinition,
    pub skills: Vec<AgentSkill>,
    pub skill_names: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RouterError {
    #[error("{0}")]
    Agent(#[from] AgentError),
    #[error("{0}")]
    Skill(#[from] SkillError),
    #[error("{0}")]
    Connector(#[from] ConnectorError),
    #[error("agent policy denied: {0}")]
    PolicyDenied(String),
}

pub struct AgentRouter {
    registry: AgentRegistry,
    connector: NervousConnector,
}

impl AgentRouter {
    pub fn load() -> Self {
        Self {
            registry: AgentRegistry::load().unwrap_or_else(|e| {
                tracing::warn!(error = %e, "agent registry load failed");
                AgentRegistry::load_from(std::path::Path::new("/nonexistent")).unwrap()
            }),
            connector: NervousConnector::load(),
        }
    }

    pub fn with_registry(registry: AgentRegistry, connector: NervousConnector) -> Self {
        Self {
            registry,
            connector,
        }
    }

    pub fn registry(&self) -> &AgentRegistry {
        &self.registry
    }

    pub fn resolve(&self, agent_id: &str) -> Result<AgentRoute, RouterError> {
        let agent = self.registry.get(agent_id)?.clone();
        let loaded = load_skills_for_agent(&agent)?;
        let skill_names: Vec<String> = loaded
            .iter()
            .filter_map(|s| s.metadata.get("name").and_then(|v| v.as_str()).map(str::to_string))
            .collect();
        Ok(AgentRoute {
            agent,
            skills: loaded,
            skill_names,
        })
    }

    /// Nervous reasoning + agent policy gate (before rmngd IPC).
    pub async fn ask(&self, agent_id: &str, prompt: &str) -> Result<CoreIntent, RouterError> {
        let route = self.resolve(agent_id)?;
        let primary_skill = route.skill_names.first().map(|s| s.as_str());
        let primary_skill_body = route.skills.first();

        let intent = self
            .connector
            .reason_core_with_agent(prompt, Some(&route.agent), primary_skill, primary_skill_body, &route.skills)
            .await?;

        route
            .agent
            .allows_core_intent(&intent)
            .map_err(RouterError::PolicyDenied)?;

        Ok(intent)
    }

    /// Validate an intent against agent policy (for testing / dry-run).
    pub fn validate_intent(agent: &AgentDefinition, intent: &CoreIntent) -> Result<(), RouterError> {
        agent
            .allows_core_intent(intent)
            .map_err(RouterError::PolicyDenied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmng_core::CoreIntent;
    use std::path::PathBuf;

    fn repo_keeper() -> AgentDefinition {
        AgentRegistry::load_from(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../definitions"))
            .unwrap()
            .get("repo-keeper")
            .unwrap()
            .clone()
    }

    #[test]
    fn repo_keeper_allows_git_status_intent() {
        let agent = repo_keeper();
        let intent = CoreIntent::ToolExecute {
            target: "git.status".into(),
            parameters: serde_json::json!({}),
            metadata: None,
        };
        assert!(AgentRouter::validate_intent(&agent, &intent).is_ok());
    }

    #[test]
    fn repo_keeper_denies_kernel_build() {
        let agent = repo_keeper();
        let intent = CoreIntent::ToolExecute {
            target: "kernel.build".into(),
            parameters: serde_json::json!({}),
            metadata: None,
        };
        assert!(AgentRouter::validate_intent(&agent, &intent).is_err());
    }
}