use crate::agent::AgentDefinition;
use crate::mock::mock_core_intent;
use crate::nervous_audit::log_nervous_event;
use crate::providers::{default_model, provider_label, LlmBackend, LlmReasonContext, ProviderError};
use crate::skill::{assemble_prompt_full, AgentSkill};
use rmng_core::session::{LlmCallRecord, SessionStore};
use rmng_core::AgentSession;
use rmng_core::{CoreIntent, RmngConfig};
use std::time::Instant;

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
    #[error("all LLM providers in fallback chain failed: {0}")]
    FallbackExhausted(String),
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

        let chain = self
            .config
            .resolved_llm_chain_for_agent(agent.map(|a| a as &dyn rmng_core::AgentLlmOverride));

        if chain.len() == 1 && chain[0].config.is_mock() {
            return Ok(mock_core_intent(
                prompt,
                skill_name,
                skill.map(|s| s.instructions.as_str()),
                agent,
                session,
            ));
        }

        let mut errors: Vec<String> = Vec::new();
        for (idx, entry) in chain.iter().enumerate() {
            if entry.config.is_mock() {
                continue;
            }
            let backend = match LlmBackend::from_config(&entry.config) {
                Ok(Some(b)) => b,
                Ok(None) => continue,
                Err(e) => {
                    errors.push(format!("{}: misconfigured ({e})", entry.label));
                    continue;
                }
            };

            let started = Instant::now();
            match backend.reason_core(&assembled, &llm_ctx).await {
                Ok(intent) => {
                    let latency_ms = started.elapsed().as_millis() as u64;
                    if idx > 0 {
                        log_nervous_event(
                            "nervous.llm_fallback",
                            "success",
                            Some(&format!(
                                "used {} after {} prior failure(s)",
                                entry.label,
                                idx
                            )),
                        );
                    }
                    let model = entry
                        .config
                        .model
                        .clone()
                        .unwrap_or_else(|| default_model(entry.config.llm_provider));
                    self.record_llm_call(
                        session,
                        agent.map(|a| a.id.as_str()),
                        &entry.label,
                        backend.id(),
                        &model,
                        latency_ms,
                        None,
                        None,
                    );
                    return Ok(intent);
                }
                Err(e) if e.warrants_provider_fallback() && idx + 1 < chain.len() => {
                    log_nervous_event(
                        "nervous.llm_fallback",
                        "retry",
                        Some(&format!(
                            "{} failed [{:?}]: {e}; trying next profile",
                            entry.label,
                            e.kind()
                        )),
                    );
                    errors.push(format!("{}: {e}", entry.label));
                }
                Err(e) => {
                    errors.push(format!("{}: {e}", entry.label));
                    if idx + 1 < chain.len() {
                        continue;
                    }
                    return Err(ConnectorError::Adapter(e));
                }
            }
        }

        if errors.is_empty() {
            return Ok(mock_core_intent(
                prompt,
                skill_name,
                skill.map(|s| s.instructions.as_str()),
                agent,
                session,
            ));
        }
        Err(ConnectorError::FallbackExhausted(errors.join("; ")))
    }

    fn record_llm_call(
        &self,
        session: Option<&AgentSession>,
        agent_id: Option<&str>,
        profile_label: &str,
        provider: &str,
        model: &str,
        latency_ms: u64,
        prompt_tokens: Option<u32>,
        completion_tokens: Option<u32>,
    ) {
        let Some(sess) = session else {
            return;
        };
        let store = SessionStore::default_store();
        let Ok(mut loaded) = store.load(&sess.id) else {
            return;
        };
        let record = LlmCallRecord {
            timestamp: chrono::Utc::now(),
            agent_id: agent_id.map(str::to_string),
            provider: provider.to_string(),
            model: model.to_string(),
            profile_label: profile_label.to_string(),
            latency_ms,
            prompt_tokens,
            completion_tokens,
        };
        if let Err(e) = store.record_llm_call(&mut loaded, record) {
            tracing::warn!(error = %e, "llm metrics write failed");
        }
    }

    pub fn provider_label(&self) -> &'static str {
        provider_label(self.config.resolved_llm().llm_provider)
    }
}