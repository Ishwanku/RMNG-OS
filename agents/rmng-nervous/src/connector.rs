use crate::agent::AgentDefinition;
use crate::mock::mock_core_intent;
use crate::providers::{provider_label, LlmBackend, LlmReasonContext, ProviderError};
use crate::skill::{assemble_prompt_full, AgentSkill};
use rmng_core::AgentSession;
use rmng_core::{CoreIntent, RmngConfig};

#[derive(Debug, thiserror::Error)]
pub enum ConnectorError {
    #[error("provider not implemented: {0}")]
    NotImplemented(String),
    #[error("provider misconfigured: {0}")]
    Misconfigured(String),
    #[error("nervous adapter error: {0}")]
    Adapter(#[from] ProviderError),
    #[error("runtime error: {0}")]
    Runtime(#[from] rmng_core::RmngError),
}

pub struct NervousConnector {
    config: RmngConfig,
}

impl NervousConnector {
    pub fn from_config(config: RmngConfig) -> Self {
        Self { config }
    }

    pub fn load() -> Self {
        Self::from_config(RmngConfig::load())
    }

    pub fn config(&self) -> &RmngConfig {
        &self.config
    }

    /// Resolve user prompt (+ optional skill) to a v2 `CoreIntent`. Never executes tools.
    pub async fn reason_core(
        &self,
        prompt: &str,
        skill_name: Option<&str>,
        skill: Option<&AgentSkill>,
    ) -> Result<CoreIntent, ConnectorError> {
        self.reason_core_with_session(prompt, None, None, skill_name, skill, &[])
            .await
    }

    /// Agent-aware reasoning with narrowed tool context in the prompt.
    pub async fn reason_core_with_agent(
        &self,
        prompt: &str,
        agent: Option<&AgentDefinition>,
        skill_name: Option<&str>,
        skill: Option<&AgentSkill>,
        extra_skills: &[AgentSkill],
    ) -> Result<CoreIntent, ConnectorError> {
        self.reason_core_with_session(prompt, agent, None, skill_name, skill, extra_skills)
            .await
    }

    /// Session-aware reasoning — injects shared context when session is active.
    pub async fn reason_core_with_session(
        &self,
        prompt: &str,
        agent: Option<&AgentDefinition>,
        session: Option<&AgentSession>,
        skill_name: Option<&str>,
        skill: Option<&AgentSkill>,
        extra_skills: &[AgentSkill],
    ) -> Result<CoreIntent, ConnectorError> {
        let assembled = assemble_prompt_full(agent, extra_skills, skill, session, prompt);

        let llm_ctx = LlmReasonContext {
            session_id: session.map(|s| s.id.as_str()),
            agent_id: agent.map(|a| a.id.as_str()),
            skill_name,
        };

        let llm = self.config.resolved_llm();
        if llm.is_mock() {
            return Ok(mock_core_intent(
                prompt,
                skill_name,
                skill.map(|s| s.instructions.as_str()),
                agent,
                session,
            ));
        }

        let backend = LlmBackend::from_config(&llm)?;
        match backend {
            Some(b) => Ok(b.reason_core(&assembled, &llm_ctx).await?),
            None => Ok(mock_core_intent(
                prompt,
                skill_name,
                skill.map(|s| s.instructions.as_str()),
                agent,
                session,
            )),
        }
    }

    pub fn provider_label(&self) -> &'static str {
        provider_label(self.config.resolved_llm().llm_provider)
    }
}