use crate::agent::{AgentDefinition, AgentError, AgentRegistry};
use crate::layer::AgentLayer;
use crate::skill::{load_skills_for_agent, AgentSkill, SkillError};
use crate::{ConnectorError, NervousConnector};
use rmng_core::session::SessionStore;
use rmng_core::CoreIntent;

/// Resolved routing context for an agent invocation.
#[derive(Debug, Clone)]
pub struct AgentRoute {
    pub agent: AgentDefinition,
    pub skills: Vec<AgentSkill>,
    pub skill_names: Vec<String>,
}

/// Result of layer-aware routing — direct intent or delegated handoff.
#[derive(Debug, Clone)]
pub enum RouteOutcome {
    Direct {
        agent_id: String,
        intent: CoreIntent,
    },
    Handoff {
        from_agent: String,
        to_agent: String,
        from_layer: AgentLayer,
        to_layer: AgentLayer,
        intent: CoreIntent,
        reason: String,
    },
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
    #[error("session error: {0}")]
    Session(String),
    #[error("orchestration requires L4 agent, got {0}")]
    NotOrchestrator(String),
}

pub struct AgentRouter {
    registry: AgentRegistry,
    connector: NervousConnector,
    sessions: SessionStore,
}

impl AgentRouter {
    pub fn load() -> Self {
        Self {
            registry: AgentRegistry::load().unwrap_or_else(|e| {
                tracing::warn!(error = %e, "agent registry load failed");
                AgentRegistry::load_from(std::path::Path::new("/nonexistent")).unwrap()
            }),
            connector: NervousConnector::load(),
            sessions: SessionStore::default_store(),
        }
    }

    pub fn with_registry(registry: AgentRegistry, connector: NervousConnector) -> Self {
        Self::with_session_store(registry, connector, SessionStore::default_store())
    }

    pub fn with_session_store(
        registry: AgentRegistry,
        connector: NervousConnector,
        sessions: SessionStore,
    ) -> Self {
        Self {
            registry,
            connector,
            sessions,
        }
    }

    pub fn registry(&self) -> &AgentRegistry {
        &self.registry
    }

    pub fn sessions(&self) -> &SessionStore {
        &self.sessions
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
        let outcome = self.ask_routed(None, agent_id, prompt).await?;
        Ok(outcome.intent())
    }

    /// Layer-aware ask with optional session persistence.
    pub async fn ask_routed(
        &self,
        session_id: Option<&str>,
        agent_id: &str,
        prompt: &str,
    ) -> Result<RouteOutcome, RouterError> {
        let route = self.resolve(agent_id)?;
        if route.agent.layer == AgentLayer::L4 {
            return self
                .orchestrate(session_id, &route, prompt)
                .await;
        }
        let intent = self.reason_for_route(session_id, &route, prompt).await?;
        route.agent.allows_core_intent(&intent).map_err(RouterError::PolicyDenied)?;
        if let Some(sid) = session_id {
            self.touch_session(sid, &route.agent, prompt)?;
        }
        Ok(RouteOutcome::Direct {
            agent_id: agent_id.to_string(),
            intent,
        })
    }

    /// Explicit handoff from one agent to another within a session.
    pub async fn handoff(
        &self,
        session_id: &str,
        from_id: &str,
        to_id: &str,
        prompt: &str,
        reason: &str,
    ) -> Result<RouteOutcome, RouterError> {
        let from = self.registry.get(from_id)?.clone();
        let to = self.registry.get(to_id)?.clone();
        from.validate_handoff_to(&to)?;
        let route = self.resolve(to_id)?;
        let intent = self.reason_for_route(Some(session_id), &route, prompt).await?;
        to.allows_core_intent(&intent).map_err(RouterError::PolicyDenied)?;

        let mut session = self
            .sessions
            .load(session_id)
            .map_err(|e| RouterError::Session(e.to_string()))?;
        self.sessions
            .record_handoff(
                &mut session,
                from_id,
                from.layer.as_str(),
                to_id,
                to.layer.as_str(),
                reason,
                Some(prompt),
            )
            .map_err(|e| RouterError::Session(e.to_string()))?;
        self.sessions
            .set_active_agent(&mut session, to.layer.as_str(), to_id, to.layer.as_str())
            .map_err(|e| RouterError::Session(e.to_string()))?;

        Ok(RouteOutcome::Handoff {
            from_agent: from_id.to_string(),
            to_agent: to_id.to_string(),
            from_layer: from.layer,
            to_layer: to.layer,
            intent,
            reason: reason.to_string(),
        })
    }

    /// Multi-hop handoff chain (e.g. L4 → L3 → L2). Records every hop in session history.
    pub async fn handoff_chain(
        &self,
        session_id: &str,
        chain: &[String],
        prompt: &str,
        reason: &str,
    ) -> Result<RouteOutcome, RouterError> {
        if chain.len() < 2 {
            return Err(RouterError::Session(
                "handoff chain requires at least two agents".into(),
            ));
        }
        let mut last: Option<RouteOutcome> = None;
        for i in 0..chain.len() - 1 {
            let from_id = &chain[i];
            let to_id = &chain[i + 1];
            let hop_reason = if i == 0 {
                reason.to_string()
            } else {
                format!("chain hop {from_id} → {to_id}")
            };
            let outcome = self
                .handoff(session_id, from_id, to_id, prompt, &hop_reason)
                .await?;
            if let RouteOutcome::Handoff {
                from_agent,
                to_agent,
                from_layer,
                to_layer,
                reason: hop,
                ..
            } = &outcome
            {
                tracing::info!(
                    session = session_id,
                    "{from_agent} ({from_layer}) → {to_agent} ({to_layer}) — {hop}"
                );
            }
            last = Some(outcome);
        }
        last.ok_or_else(|| RouterError::Session("empty handoff chain".into()))
    }

    async fn orchestrate(
        &self,
        session_id: Option<&str>,
        route: &AgentRoute,
        prompt: &str,
    ) -> Result<RouteOutcome, RouterError> {
        let orchestrator = &route.agent;
        let plan = self.reason_for_route(session_id, route, prompt).await?;

        let (delegate_hint, reason) = match &plan {
            CoreIntent::ToolExecute { target, .. } => (target.clone(), format!("execute {target}")),
            CoreIntent::McpProxy { mcp_server, mcp_tool, .. } => (
                format!("{mcp_server}:{mcp_tool}"),
                format!("mcp {mcp_server}.{mcp_tool}"),
            ),
            CoreIntent::PlanOnly { reasoning: _, .. } => {
                if let Some(sid) = session_id {
                    self.touch_session(sid, orchestrator, prompt)?;
                }
                return Ok(RouteOutcome::Direct {
                    agent_id: orchestrator.id.clone(),
                    intent: plan,
                });
            }
        };

        let delegate = self.registry.resolve_handoff_target(orchestrator, &delegate_hint)?;
        let delegate_route = self.resolve(&delegate.id)?;
        let intent = self.reason_for_route(session_id, &delegate_route, prompt).await?;
        delegate
            .allows_core_intent(&intent)
            .map_err(RouterError::PolicyDenied)?;

        if let Some(sid) = session_id {
            let mut session = self
                .sessions
                .load(sid)
                .map_err(|e| RouterError::Session(e.to_string()))?;
            self.sessions
                .record_handoff(
                    &mut session,
                    &orchestrator.id,
                    orchestrator.layer.as_str(),
                    &delegate.id,
                    delegate.layer.as_str(),
                    &reason,
                    Some(prompt),
                )
                .map_err(|e| RouterError::Session(e.to_string()))?;
            self.sessions
                .set_active_agent(
                    &mut session,
                    delegate.layer.as_str(),
                    &delegate.id,
                    delegate.layer.as_str(),
                )
                .map_err(|e| RouterError::Session(e.to_string()))?;
        }

        Ok(RouteOutcome::Handoff {
            from_agent: orchestrator.id.clone(),
            to_agent: delegate.id.clone(),
            from_layer: orchestrator.layer,
            to_layer: delegate.layer,
            intent,
            reason,
        })
    }

    async fn reason_for_route(
        &self,
        session_id: Option<&str>,
        route: &AgentRoute,
        prompt: &str,
    ) -> Result<CoreIntent, RouterError> {
        let primary_skill = route.skill_names.first().map(|s| s.as_str());
        let primary_skill_body = route.skills.first();
        let session = session_id
            .and_then(|sid| self.sessions.load(sid).ok());
        Ok(self
            .connector
            .reason_core_with_session(
                prompt,
                Some(&route.agent),
                session.as_ref(),
                primary_skill,
                primary_skill_body,
                &route.skills,
            )
            .await?)
    }

    /// Attach session/handoff metadata to an intent before rmngd dispatch.
    pub fn enrich_intent_metadata(
        intent: &mut CoreIntent,
        session_id: Option<&str>,
        handoff_from: Option<&str>,
    ) {
        use rmng_core::intent::Metadata;
        let patch = |meta: &mut Option<Metadata>| {
            let m = meta.get_or_insert(Metadata {
                trace_id: None,
                skill_name: None,
                session_id: None,
                handoff_from: None,
            });
            if let Some(sid) = session_id {
                m.session_id = Some(sid.to_string());
                m.trace_id = Some(sid.to_string());
            }
            if let Some(from) = handoff_from {
                m.handoff_from = Some(from.to_string());
            }
        };
        match intent {
            CoreIntent::ToolExecute { metadata, .. }
            | CoreIntent::McpProxy { metadata, .. }
            | CoreIntent::PlanOnly { metadata, .. } => patch(metadata),
        }
    }

    fn touch_session(
        &self,
        session_id: &str,
        agent: &AgentDefinition,
        prompt: &str,
    ) -> Result<(), RouterError> {
        let mut session = self
            .sessions
            .load(session_id)
            .map_err(|e| RouterError::Session(e.to_string()))?;
        session.task_state.current_prompt = Some(prompt.to_string());
        self.sessions
            .set_active_agent(
                &mut session,
                agent.layer.as_str(),
                &agent.id,
                agent.layer.as_str(),
            )
            .map_err(|e| RouterError::Session(e.to_string()))?;
        Ok(())
    }

    pub fn validate_intent(agent: &AgentDefinition, intent: &CoreIntent) -> Result<(), RouterError> {
        agent
            .allows_core_intent(intent)
            .map_err(RouterError::PolicyDenied)
    }
}

impl RouteOutcome {
    pub fn intent(&self) -> CoreIntent {
        match self {
            Self::Direct { intent, .. } | Self::Handoff { intent, .. } => intent.clone(),
        }
    }

    pub fn is_handoff(&self) -> bool {
        matches!(self, Self::Handoff { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
