use crate::agent::{AgentDefinition, AgentError, AgentRegistry};
use crate::layer::AgentLayer;
use crate::nervous_audit::log_nervous_event;
use crate::skill::{load_skills_for_agent, AgentSkill, SkillError};
use crate::{ConnectorError, NervousConnector};
use rmng_core::session::SessionStore;
use rmng_core::{CoreIntent, HandoffChainOptions, HopFailurePolicy};

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
    /// Multi-hop chain completed; final intent from last agent in chain (Sprint 23).
    HandoffChain {
        chain: Vec<String>,
        hops: Vec<HandoffHopRecord>,
        skipped_hops: Vec<SkippedHopRecord>,
        intent: CoreIntent,
        reason: String,
    },
}

/// One hop in a recorded handoff chain.
#[derive(Debug, Clone)]
pub struct HandoffHopRecord {
    pub from_agent: String,
    pub to_agent: String,
    pub reason: String,
}

/// Hop skipped during chain recovery (Sprint 25).
#[derive(Debug, Clone)]
pub struct SkippedHopRecord {
    pub hop_index: usize,
    pub from_agent: String,
    pub skipped_agent: String,
    pub error: String,
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
            if let CoreIntent::PlanOnly { .. } = &intent {
                if let Some(result) = self
                    .try_autonomous_plan_handoff(sid, agent_id, &intent, prompt, "llm suggested")
                    .await?
                {
                    return Ok(result);
                }
            }
        }

        if let Some(sid) = session_id {
            self.touch_session(sid, &route.agent, prompt)?;
        }
        Ok(RouteOutcome::Direct {
            agent_id: agent_id.to_string(),
            intent,
        })
    }

    /// Pre-validate a single handoff (Sprint 8) — agent exists, layer rules, session active.
    pub fn validate_handoff(
        &self,
        session_id: &str,
        from_id: &str,
        to_id: &str,
    ) -> Result<(), RouterError> {
        self.sessions
            .load(session_id)
            .map_err(|e| RouterError::Session(format!("session '{session_id}': {e}")))?;
        let from = self.registry.get(from_id)?;
        let to = self.registry.get(to_id)?;
        from.validate_handoff_to(to).map_err(|e| {
            RouterError::Session(format!(
                "handoff '{from_id}' → '{to_id}' rejected: {e}"
            ))
        })
    }

    /// Pre-validate upward return to orchestrator (Sprint 23 feedback loop).
    pub fn validate_handoff_return(
        &self,
        session_id: &str,
        from_id: &str,
        to_id: &str,
    ) -> Result<(), RouterError> {
        self.sessions
            .load(session_id)
            .map_err(|e| RouterError::Session(format!("session '{session_id}': {e}")))?;
        let from = self.registry.get(from_id)?;
        let to = self.registry.get(to_id)?;
        from.validate_handoff_return_to(to).map_err(|e| {
            RouterError::Session(format!(
                "handoff return '{from_id}' → '{to_id}' rejected: {e}"
            ))
        })
    }

    /// Pre-validate every hop in a handoff chain (Sprint 8).
    pub fn validate_handoff_chain(
        &self,
        session_id: &str,
        chain: &[String],
    ) -> Result<(), RouterError> {
        if chain.len() < 2 {
            return Err(RouterError::Session(
                "handoff chain requires at least two agent ids".into(),
            ));
        }
        self.sessions
            .load(session_id)
            .map_err(|e| RouterError::Session(format!("session '{session_id}': {e}")))?;
        for id in chain {
            self.registry.get(id).map_err(|e| {
                RouterError::Session(format!("chain agent '{id}' invalid: {e}"))
            })?;
        }
        for i in 0..chain.len() - 1 {
            let from_id = &chain[i];
            let to_id = &chain[i + 1];
            self.validate_handoff(session_id, from_id, to_id)?;
        }
        Ok(())
    }

    /// Return control to orchestrator after specialist work (Sprint 23).
    pub async fn handoff_return(
        &self,
        session_id: &str,
        from_id: &str,
        to_id: &str,
        prompt: &str,
        reason: &str,
    ) -> Result<RouteOutcome, RouterError> {
        self.validate_handoff_return(session_id, from_id, to_id)?;
        self.handoff_inner(session_id, from_id, to_id, prompt, reason, true)
            .await
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
        self.validate_handoff(session_id, from_id, to_id)?;
        self.handoff_inner(session_id, from_id, to_id, prompt, reason, false)
            .await
    }

    async fn handoff_inner(
        &self,
        session_id: &str,
        from_id: &str,
        to_id: &str,
        prompt: &str,
        reason: &str,
        upward_return: bool,
    ) -> Result<RouteOutcome, RouterError> {
        let from = self.registry.get(from_id)?.clone();
        let to = self.registry.get(to_id)?.clone();
        let intent = if upward_return {
            let session = self
                .sessions
                .load(session_id)
                .map_err(|e| RouterError::Session(e.to_string()))?;
            let summary = session.tool_results_summary(5);
            CoreIntent::PlanOnly {
                reasoning: format!(
                    "Specialist {from_id} returned control. Recent results:
{summary}
Return context: {prompt}"
                ),
                metadata: Some(rmng_core::intent::Metadata {
                    trace_id: Some(session_id.to_string()),
                    skill_name: None,
                    session_id: Some(session_id.to_string()),
                    handoff_from: Some(from_id.to_string()),
                    handoff_to: None,
                    handoff_chain: None,
                    handoff_return_to: None,
                    chain_id: None,
                hop_failure_policy: None,
                    hop_retry_max: None,
                }),
            }
        } else {
            let route = self.resolve(to_id)?;
            self.reason_for_route(Some(session_id), &route, prompt).await?
        };
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

        let audit_action = if upward_return {
            "nervous.handoff_return"
        } else {
            "nervous.handoff"
        };
        log_nervous_event(
            audit_action,
            "success",
            Some(&format!("session={session_id} {from_id}→{to_id} reason={reason}")),
        );

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
        self.handoff_chain_with_options(
            session_id,
            chain,
            prompt,
            reason,
            HandoffChainOptions::default(),
        )
        .await
    }

    /// Multi-hop chain with configurable hop failure policy (Sprint 25).
    pub async fn handoff_chain_with_options(
        &self,
        session_id: &str,
        chain: &[String],
        prompt: &str,
        reason: &str,
        options: HandoffChainOptions,
    ) -> Result<RouteOutcome, RouterError> {
        self.validate_handoff_chain(session_id, chain)?;
        let chain_id = session_id.to_string();
        let mut hops: Vec<HandoffHopRecord> = Vec::new();
        let mut skipped_hops: Vec<SkippedHopRecord> = Vec::new();
        let mut last_intent: Option<CoreIntent> = None;

        {
            let mut session = self
                .sessions
                .load(session_id)
                .map_err(|e| RouterError::Session(e.to_string()))?;
            self.sessions
                .set_orchestration_state(
                    &mut session,
                    serde_json::json!({
                        "chain_id": chain_id,
                        "chain": chain,
                        "hops_completed": 0,
                        "origin_agent": chain.first(),
                        "return_to": chain.first(),
                        "status": "in_progress",
                        "hop_failure_policy": format!("{:?}", options.hop_failure_policy).to_ascii_lowercase(),
                        "hop_retry_max": options.hop_retry_max,
                    }),
                )
                .map_err(|e| RouterError::Session(e.to_string()))?;
        }

        let mut i = 0usize;
        while i < chain.len().saturating_sub(1) {
            let from_id = chain[i].clone();
            let to_id = chain[i + 1].clone();
            let hop_reason = if i == 0 {
                reason.to_string()
            } else {
                format!("chain hop {from_id} → {to_id}")
            };

            match self
                .execute_chain_hop(
                    session_id,
                    &chain_id,
                    i,
                    &from_id,
                    &to_id,
                    prompt,
                    &hop_reason,
                    &options,
                )
                .await
            {
                Ok(outcome) => {
                    if let RouteOutcome::Handoff {
                        from_agent,
                        to_agent,
                        from_layer,
                        to_layer,
                        reason: hop,
                        intent,
                        ..
                    } = &outcome
                    {
                        tracing::info!(
                            session = session_id,
                            "{from_agent} ({from_layer}) → {to_agent} ({to_layer}) — {hop}"
                        );
                        log_nervous_event(
                            "nervous.handoff_chain_hop",
                            "success",
                            Some(&format!(
                                "session={session_id} hop={i} {from_agent}→{to_agent} chain_id={chain_id}"
                            )),
                        );
                        hops.push(HandoffHopRecord {
                            from_agent: from_agent.clone(),
                            to_agent: to_agent.clone(),
                            reason: hop.clone(),
                        });
                        last_intent = Some(intent.clone());
                    }
                    self.bump_chain_progress(session_id, i + 1, &to_id)?;
                    i += 1;
                }
                Err(e) => {
                    let msg = e.to_string();
                    match options.hop_failure_policy {
                        HopFailurePolicy::Abort => {
                            self.abort_chain_hop(
                                session_id, i, &from_id, &to_id, &msg, "abort", &options,
                            )?;
                            return Err(e);
                        }
                        HopFailurePolicy::Retry => {
                            // Retries exhausted inside execute_chain_hop; treat as abort.
                            self.abort_chain_hop(
                                session_id, i, &from_id, &to_id, &msg, "abort_after_retry", &options,
                            )?;
                            return Err(e);
                        }
                        HopFailurePolicy::Skip => {
                            self.log_hop_policy_decision(
                                session_id,
                                i,
                                &from_id,
                                &to_id,
                                &options,
                                "skip",
                                &msg,
                                None,
                            )?;
                            if let Ok(mut session) = self.sessions.load(session_id) {
                                let _ = self.sessions.record_skipped_hop(
                                    &mut session,
                                    i,
                                    &from_id,
                                    &to_id,
                                    &msg,
                                );
                            }
                            skipped_hops.push(SkippedHopRecord {
                                hop_index: i,
                                from_agent: from_id.clone(),
                                skipped_agent: to_id.clone(),
                                error: msg.clone(),
                            });
                            log_nervous_event(
                                "nervous.handoff_chain_hop",
                                "skipped",
                                Some(&format!(
                                    "session={session_id} hop={i} {from_id}→{to_id} policy=skip error={msg}"
                                )),
                            );

                            if i + 2 < chain.len() {
                                let shortcut_to = chain[i + 2].clone();
                                let shortcut_reason =
                                    format!("skip recovery {from_id} → {shortcut_to} (skipped {to_id})");
                                match self
                                    .execute_chain_hop(
                                        session_id,
                                        &chain_id,
                                        i,
                                        &from_id,
                                        &shortcut_to,
                                        prompt,
                                        &shortcut_reason,
                                        &options,
                                    )
                                    .await
                                {
                                    Ok(outcome) => {
                                        if let RouteOutcome::Handoff {
                                            from_agent,
                                            to_agent,
                                            reason: hop,
                                            intent,
                                            ..
                                        } = &outcome
                                        {
                                            log_nervous_event(
                                                "nervous.handoff_chain_hop",
                                                "success",
                                                Some(&format!(
                                                    "session={session_id} hop={i} shortcut {from_agent}→{to_agent} chain_id={chain_id}"
                                                )),
                                            );
                                            hops.push(HandoffHopRecord {
                                                from_agent: from_agent.clone(),
                                                to_agent: to_agent.clone(),
                                                reason: hop.clone(),
                                            });
                                            last_intent = Some(intent.clone());
                                        }
                                        self.bump_chain_progress(session_id, i + 2, &shortcut_to)?;
                                        i += 2;
                                    }
                                    Err(shortcut_err) => {
                                        let shortcut_msg = shortcut_err.to_string();
                                        self.abort_chain_hop(
                                            session_id,
                                            i,
                                            &from_id,
                                            &shortcut_to,
                                            &shortcut_msg,
                                            "abort_after_skip_shortcut_failed",
                                            &options,
                                        )?;
                                        return Err(shortcut_err);
                                    }
                                }
                            } else {
                                // No further agents; partial completion at from_id.
                                tracing::warn!(
                                    session = session_id,
                                    hop = i,
                                    "chain completed with skipped terminal hop"
                                );
                                break;
                            }
                        }
                    }
                }
            }
        }

        let final_status = if skipped_hops.is_empty() {
            "completed"
        } else {
            "completed_with_skips"
        };

        {
            let mut session = self
                .sessions
                .load(session_id)
                .map_err(|e| RouterError::Session(e.to_string()))?;
            if let Some(orch) = session.shared_context.get_mut("orchestration") {
                if let Some(obj) = orch.as_object_mut() {
                    obj.insert("status".into(), serde_json::json!(final_status));
                    obj.insert(
                        "hops_completed".into(),
                        serde_json::json!(hops.len()),
                    );
                }
            }
            self.sessions
                .save(&session)
                .map_err(|e| RouterError::Session(e.to_string()))?;
        }

        log_nervous_event(
            "nervous.handoff_chain_complete",
            "success",
            Some(&format!(
                "session={session_id} hops={} skipped={} status={final_status} chain_id={chain_id}",
                hops.len(),
                skipped_hops.len()
            )),
        );

        let intent = last_intent.ok_or_else(|| RouterError::Session("empty handoff chain".into()))?;
        Ok(RouteOutcome::HandoffChain {
            chain: chain.to_vec(),
            hops,
            skipped_hops,
            intent,
            reason: reason.to_string(),
        })
    }

    async fn execute_chain_hop(
        &self,
        session_id: &str,
        chain_id: &str,
        hop_index: usize,
        from_id: &str,
        to_id: &str,
        prompt: &str,
        hop_reason: &str,
        options: &HandoffChainOptions,
    ) -> Result<RouteOutcome, RouterError> {
        let mut attempt = 0u32;
        loop {
            match self
                .handoff(session_id, from_id, to_id, prompt, hop_reason)
                .await
            {
                Ok(outcome) => return Ok(outcome),
                Err(e) => {
                    attempt += 1;
                    let msg = e.to_string();
                    if options.hop_failure_policy == HopFailurePolicy::Retry
                        && attempt <= options.hop_retry_max
                    {
                        self.log_hop_policy_decision(
                            session_id,
                            hop_index,
                            from_id,
                            to_id,
                            options,
                            "retry",
                            &msg,
                            Some(attempt),
                        )?;
                        log_nervous_event(
                            "nervous.handoff_chain_hop",
                            "retry",
                            Some(&format!(
                                "session={session_id} hop={hop_index} {from_id}→{to_id} attempt={attempt}/{} chain_id={chain_id} error={msg}",
                                options.hop_retry_max
                            )),
                        );
                        continue;
                    }
                    log_nervous_event(
                        "nervous.handoff_chain_hop",
                        "failed",
                        Some(&format!(
                            "session={session_id} hop={hop_index} {from_id}→{to_id} chain_id={chain_id} error={msg}"
                        )),
                    );
                    return Err(e);
                }
            }
        }
    }

    fn bump_chain_progress(
        &self,
        session_id: &str,
        hops_completed: usize,
        active_agent: &str,
    ) -> Result<(), RouterError> {
        let mut session = self
            .sessions
            .load(session_id)
            .map_err(|e| RouterError::Session(e.to_string()))?;
        if let Some(orch) = session.shared_context.get_mut("orchestration") {
            if let Some(obj) = orch.as_object_mut() {
                obj.insert("hops_completed".into(), serde_json::json!(hops_completed));
                obj.insert("active_agent".into(), serde_json::json!(active_agent));
            }
        }
        self.sessions
            .save(&session)
            .map_err(|e| RouterError::Session(e.to_string()))
    }

    fn log_hop_policy_decision(
        &self,
        session_id: &str,
        hop_index: usize,
        from_id: &str,
        to_id: &str,
        options: &HandoffChainOptions,
        action: &str,
        error: &str,
        attempt: Option<u32>,
    ) -> Result<(), RouterError> {
        if let Ok(mut session) = self.sessions.load(session_id) {
            let policy = format!("{:?}", options.hop_failure_policy).to_ascii_lowercase();
            let _ = self.sessions.record_hop_policy_decision(
                &mut session,
                hop_index,
                from_id,
                to_id,
                &policy,
                action,
                error,
                attempt,
            );
        }
        log_nervous_event(
            "nervous.handoff_chain_policy",
            action,
            Some(&format!(
                "session={session_id} hop={hop_index} {from_id}→{to_id} policy={} action={action} error={error}",
                format!("{:?}", options.hop_failure_policy).to_ascii_lowercase()
            )),
        );
        Ok(())
    }

    fn abort_chain_hop(
        &self,
        session_id: &str,
        hop_index: usize,
        from_id: &str,
        to_id: &str,
        error: &str,
        action: &str,
        options: &HandoffChainOptions,
    ) -> Result<(), RouterError> {
        self.log_hop_policy_decision(
            session_id,
            hop_index,
            from_id,
            to_id,
            options,
            action,
            error,
            None,
        )?;
        if let Ok(mut session) = self.sessions.load(session_id) {
            let _ = self.sessions.record_chain_failure(
                &mut session,
                hop_index,
                from_id,
                to_id,
                error,
            );
            let _ = self.sessions.record_hop_error(
                &mut session,
                hop_index,
                from_id,
                to_id,
                error,
                Some(action),
            );
        }
        Ok(())
    }

    fn chain_options_from_plan(plan: &CoreIntent) -> HandoffChainOptions {
        plan.metadata()
            .map(HandoffChainOptions::from_metadata)
            .unwrap_or_default()
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
                    if let Some(result) = self
                        .try_autonomous_plan_handoff(
                            sid,
                            &orchestrator.id,
                            &plan,
                            prompt,
                            "llm orchestration",
                        )
                        .await?
                    {
                        return Ok(result);
                    }
                }
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


    /// Autonomous handoff from plan.only metadata: return_to, chain, or single hop (Sprint 23).
    async fn try_autonomous_plan_handoff(
        &self,
        session_id: &str,
        agent_id: &str,
        plan: &CoreIntent,
        prompt: &str,
        context: &str,
    ) -> Result<Option<RouteOutcome>, RouterError> {
        let CoreIntent::PlanOnly { .. } = plan else {
            return Ok(None);
        };

        if let Some(return_to) = Self::metadata_handoff_return_to(plan) {
            if return_to != agent_id {
                if let Err(e) = self.validate_handoff_return(session_id, agent_id, return_to) {
                    tracing::warn!(session = session_id, error = %e, "handoff_return_to rejected");
                    return Err(e);
                }
                tracing::info!(
                    session = session_id,
                    from = agent_id,
                    to = return_to,
                    "{context} handoff_return_to"
                );
                return Ok(Some(
                    self.handoff_return(
                        session_id,
                        agent_id,
                        return_to,
                        prompt,
                        &format!("{context} return to orchestrator"),
                    )
                    .await?,
                ));
            }
        }

        if let Some(chain) = Self::metadata_handoff_chain(plan) {
            if let Err(e) = self.validate_handoff_chain(session_id, &chain) {
                tracing::warn!(session = session_id, error = %e, "handoff_chain rejected");
                return Err(e);
            }
            tracing::info!(
                session = session_id,
                from = agent_id,
                chain = ?chain,
                "{context} handoff_chain"
            );
            let options = Self::chain_options_from_plan(plan);
            return Ok(Some(
                self.handoff_chain_with_options(
                    session_id,
                    &chain,
                    prompt,
                    &format!("{context} handoff chain"),
                    options,
                )
                .await?,
            ));
        }

        if let Some(target) = Self::handoff_target(plan) {
            if target != agent_id {
                if let Err(e) = self.validate_handoff(session_id, agent_id, target) {
                    tracing::warn!(session = session_id, error = %e, "handoff_to rejected");
                    return Err(e);
                }
                tracing::info!(
                    session = session_id,
                    from = agent_id,
                    to = target,
                    "{context} handoff_to"
                );
                log_nervous_event(
                    "nervous.handoff",
                    "success",
                    Some(&format!("session={session_id} {agent_id}→{target}")),
                );
                return Ok(Some(
                    self.handoff(session_id, agent_id, target, prompt, &format!("{context} handoff"))
                        .await?,
                ));
            }
        }

        Ok(None)
    }

    fn handoff_target(intent: &CoreIntent) -> Option<&str> {
        intent
            .metadata()
            .and_then(|m| m.handoff_to.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }

    fn metadata_handoff_return_to(intent: &CoreIntent) -> Option<&str> {
        intent
            .metadata()
            .and_then(|m| m.handoff_return_to.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }

    fn metadata_handoff_chain(intent: &CoreIntent) -> Option<Vec<String>> {
        intent.metadata().and_then(|m| m.handoff_chain.as_ref()).and_then(|chain| {
            let ids: Vec<String> = chain
                .iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if ids.len() >= 2 {
                Some(ids)
            } else {
                None
            }
        })
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
                handoff_to: None,
                handoff_chain: None,
                handoff_return_to: None,
                chain_id: None,
                hop_failure_policy: None,
                    hop_retry_max: None,
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
        session.mark_active(prompt);
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
            Self::Direct { intent, .. }
            | Self::Handoff { intent, .. }
            | Self::HandoffChain { intent, .. } => intent.clone(),
        }
    }

    pub fn is_handoff(&self) -> bool {
        matches!(self, Self::Handoff { .. } | Self::HandoffChain { .. })
    }

    pub fn handoff_from_agent(&self) -> Option<&str> {
        match self {
            Self::Handoff { from_agent, .. } => Some(from_agent),
            Self::HandoffChain { chain, .. } => chain.first().map(|s| s.as_str()),
            _ => None,
        }
    }

    pub fn final_agent(&self) -> Option<&str> {
        match self {
            Self::Direct { agent_id, .. } => Some(agent_id),
            Self::Handoff { to_agent, .. } => Some(to_agent),
            Self::HandoffChain { chain, .. } => chain.last().map(|s| s.as_str()),
        }
    }

    /// Human-readable chain summary for CLI (Sprint 25).
    pub fn chain_outcome_summary(&self) -> Option<String> {
        match self {
            Self::HandoffChain {
                chain,
                hops,
                skipped_hops,
                reason,
                ..
            } => {
                let mut lines = vec![format!("handoff-chain ({reason}): {}", chain.join(" → "))];
                for hop in hops {
                    lines.push(format!(
                        "  ok: {} → {} — {}",
                        hop.from_agent, hop.to_agent, hop.reason
                    ));
                }
                for skip in skipped_hops {
                    lines.push(format!(
                        "  skipped hop {}: {} skipped {} — {}",
                        skip.hop_index, skip.from_agent, skip.skipped_agent, skip.error
                    ));
                }
                Some(lines.join("
"))
            }
            Self::Handoff {
                from_agent,
                to_agent,
                from_layer,
                to_layer,
                reason,
                ..
            } => Some(format!(
                "handoff: {from_agent} ({from_layer}) → {to_agent} ({to_layer}) — {reason}"
            )),
            _ => None,
        }
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
    fn handoff_target_reads_metadata() {
        let intent = CoreIntent::PlanOnly {
            reasoning: "delegate".into(),
            metadata: Some(rmng_core::intent::Metadata {
                trace_id: None,
                skill_name: None,
                session_id: Some("sess-1".into()),
                handoff_from: None,
                handoff_to: Some("repo-keeper".into()),
                handoff_chain: None,
                handoff_return_to: None,
                chain_id: None,
                hop_failure_policy: None,
                hop_retry_max: None,
            }),
        };
        assert_eq!(AgentRouter::handoff_target(&intent), Some("repo-keeper"));
    }

    #[test]
    fn handoff_chain_reads_metadata() {
        let intent = CoreIntent::PlanOnly {
            reasoning: "delegate chain".into(),
            metadata: Some(rmng_core::intent::Metadata {
                trace_id: None,
                skill_name: None,
                session_id: Some("sess-1".into()),
                handoff_from: None,
                handoff_to: None,
                handoff_chain: Some(vec![
                    "swarm-coordinator".into(),
                    "repo-keeper".into(),
                    "runtime-executor".into(),
                ]),
            handoff_return_to: None,
                chain_id: None,
                hop_failure_policy: None,
                hop_retry_max: None,
            }),
        };
        let chain = AgentRouter::metadata_handoff_chain(&intent).unwrap();
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[1], "repo-keeper");
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

    fn test_router_with_session(store: SessionStore) -> AgentRouter {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../definitions");
        let registry = AgentRegistry::load_from(root).expect("registry");
        let connector = NervousConnector::from_config(rmng_core::RmngConfig::default());
        AgentRouter::with_session_store(registry, connector, store)
    }

    #[test]
    fn validate_handoff_rejects_unknown_agent() {
        let dir = std::env::temp_dir().join(format!("rmng-val-{}", uuid::Uuid::new_v4()));
        let store = SessionStore::new(&dir);
        let session = store.create().expect("create");
        let router = test_router_with_session(store);
        let err = router
            .validate_handoff(&session.id, "swarm-coordinator", "no-such-agent")
            .unwrap_err();
        assert!(err.to_string().contains("no-such-agent"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn validate_handoff_rejects_layer_violation() {
        let dir = std::env::temp_dir().join(format!("rmng-val-{}", uuid::Uuid::new_v4()));
        let store = SessionStore::new(&dir);
        let session = store.create().expect("create");
        let router = test_router_with_session(store);
        let err = router
            .validate_handoff(&session.id, "repo-keeper", "swarm-coordinator")
            .unwrap_err();
        assert!(err.to_string().contains("rejected"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn validate_handoff_chain_requires_two_agents() {
        let dir = std::env::temp_dir().join(format!("rmng-val-{}", uuid::Uuid::new_v4()));
        let store = SessionStore::new(&dir);
        let session = store.create().expect("create");
        let router = test_router_with_session(store);
        let err = router
            .validate_handoff_chain(&session.id, &["swarm-coordinator".into()])
            .unwrap_err();
        assert!(err.to_string().contains("at least two"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn validate_handoff_chain_accepts_valid_hops() {
        let dir = std::env::temp_dir().join(format!("rmng-val-{}", uuid::Uuid::new_v4()));
        let store = SessionStore::new(&dir);
        let session = store.create().expect("create");
        let router = test_router_with_session(store);
        router
            .validate_handoff_chain(
                &session.id,
                &[
                    "swarm-coordinator".into(),
                    "repo-keeper".into(),
                    "runtime-executor".into(),
                ],
            )
            .expect("valid chain");
        let _ = std::fs::remove_dir_all(dir);
    }
}
