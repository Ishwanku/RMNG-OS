use crate::agent::AgentDefinition;
use crate::chain::run_fallback_chain;
use crate::mock::mock_core_intent;
use crate::nervous_audit::{log_llm_telemetry, log_nervous_event, log_system_event};
use crate::providers::{
    allow_request, default_model, provider_label, record_failure, record_success, LlmBackend,
    LlmReasonContext, LlmUsage, ProviderError,
};
use crate::skill::{assemble_prompt_full, AgentSkill};
use rmng_core::session::{LlmCallRecord, SessionStore};
use rmng_core::AgentSession;
use rmng_core::{check_budget_from_audit_for_agent, BudgetLevel, CoreIntent, RmngConfig};
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

        let chain: Vec<_> = self
            .config
            .resolved_llm_chain_for_agent(agent.map(|a| a as &dyn rmng_core::AgentLlmOverride))
            .into_iter()
            .filter(|e| !e.config.is_mock())
            .collect();

        if chain.is_empty() {
            return Ok(mock_core_intent(
                prompt,
                skill_name,
                skill.map(|s| s.instructions.as_str()),
                agent,
                session,
            ));
        }

        let budget_agent = agent.map(|a| a.id.as_str());
        let agent_cap = agent.and_then(|a| a.daily_budget_usd);
        if let Some(budget) =
            check_budget_from_audit_for_agent(&self.config, budget_agent, agent_cap)
        {
            if budget.level == BudgetLevel::Warn {
                log_system_event(
                    "nervous.budget_warn",
                    "warn",
                    Some(&budget.message),
                );
            }
            if !budget.allowed {
                log_system_event(
                    "nervous.budget_deny",
                    "denied",
                    Some(&budget.message),
                );
                return Err(ConnectorError::Misconfigured(format!(
                    "LLM budget exceeded: {}",
                    budget.message
                )));
            }
        }

        let chain_len = chain.len();
        let session_id = session.map(|s| s.id.clone());
        let agent_id = agent.map(|a| a.id.clone());

        let run_result = run_fallback_chain(
            chain_len,
            |idx| {
                let entry = chain[idx].clone();
                let assembled = assembled.clone();
                let llm_ctx = llm_ctx.clone();
                let session_id = session_id.clone();
                let agent_id = agent_id.clone();
                async move {
                    let backend = LlmBackend::from_config(&entry.config)?
                        .ok_or_else(|| {
                            ProviderError::Misconfigured(format!(
                                "{} misconfigured",
                                entry.label
                            ))
                        })?;
                    let provider_id = backend.id();
                    if !allow_request(provider_id) {
                        return Err(ProviderError::api(
                            provider_id,
                            429,
                            "circuit breaker open — skipping provider",
                        ));
                    }
                    let started = Instant::now();
                    match backend.reason_core(&assembled, &llm_ctx).await {
                        Ok(result) => {
                            record_success(provider_id);
                            let latency_ms = started.elapsed().as_millis() as u64;
                            let model = entry
                                .config
                                .model
                                .clone()
                                .unwrap_or_else(|| default_model(entry.config.llm_provider));
                            let usage = &result.usage;
                            log_llm_telemetry(
                                provider_id,
                                &model,
                                &entry.label,
                                agent_id.as_deref(),
                                session_id.as_deref(),
                                latency_ms,
                                usage.prompt_tokens,
                                usage.completion_tokens,
                                usage.estimated_cost_usd,
                                idx as u32,
                            );
                            if idx > 0 {
                                log_nervous_event(
                                    "nervous.llm_fallback",
                                    "success",
                                    Some(&format!(
                                        "used {} after {} prior failure(s)",
                                        entry.label, idx
                                    )),
                                );
                            }
                            Self::persist_llm_call(
                                session_id.as_deref(),
                                agent_id.as_deref(),
                                &entry.label,
                                provider_id,
                                &model,
                                latency_ms,
                                usage,
                                idx as u32,
                            );
                            Ok(result.intent)
                        }
                        Err(e) => {
                            record_failure(provider_id, e.kind());
                            Err(e)
                        }
                    }
                }
            },
            |e: &ProviderError| e.warrants_provider_fallback(),
        )
        .await
        .map_err(|exhausted| {
            if exhausted.errors.is_empty() {
                ConnectorError::FallbackExhausted("no providers attempted".into())
            } else {
                ConnectorError::FallbackExhausted(exhausted.errors.join("; "))
            }
        })?;

        for (i, msg) in run_result.prior_failures.iter().enumerate() {
            log_nervous_event(
                "nervous.llm_fallback",
                "retry",
                Some(&format!("prior failure {}: {msg}", i + 1)),
            );
        }

        Ok(run_result.value)
    }

    fn persist_llm_call(
        session_id: Option<&str>,
        agent_id: Option<&str>,
        profile_label: &str,
        provider: &str,
        model: &str,
        latency_ms: u64,
        usage: &LlmUsage,
        fallback_index: u32,
    ) {
        let Some(sid) = session_id else {
            return;
        };
        let store = SessionStore::default_store();
        let Ok(mut loaded) = store.load(sid) else {
            return;
        };
        let record = LlmCallRecord {
            timestamp: chrono::Utc::now(),
            agent_id: agent_id.map(str::to_string),
            provider: provider.to_string(),
            model: model.to_string(),
            profile_label: profile_label.to_string(),
            latency_ms,
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
            estimated_cost_usd: usage.estimated_cost_usd,
            fallback_index,
        };
        if let Err(e) = store.record_llm_call(&mut loaded, record) {
            tracing::warn!(error = %e, "llm metrics write failed");
        }
    }

    pub fn provider_label(&self) -> &'static str {
        provider_label(self.config.resolved_llm().llm_provider)
    }
}