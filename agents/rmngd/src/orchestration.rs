//! Daemon-side auto-continue using AutoContinueLoop (Sprint 26–27).

use crate::continuation_locks::SessionContinuationLocks;
use rmng_core::{
    persist_dispatch_to_session, CoreIntent, HandleResponse, OrchestrationContinueResponse,
    RmngConfig, Runtime, SessionStore, ContinuationStatus,
};
use rmng_nervous::{
    AgentRouter, AutoContinueLoop, AutoContinueStep, AutoContinueStopReason, RouteOutcome,
};
use std::time::Duration;
use tracing::{info, warn};

/// Default max auto-continue steps when not specified in session (Sprint 26).
pub const DEFAULT_DAEMON_MAX_STEPS: u32 = 3;

pub fn max_steps_from_env() -> u32 {
    std::env::var("RMNG_AUTO_CONTINUE_MAX_STEPS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_DAEMON_MAX_STEPS)
}

fn apply_default_failure_policy(store: &SessionStore, session_id: &str, policy: &str) {
    let Ok(mut session) = store.load(session_id) else {
        return;
    };
    let Some(orch) = session.shared_context.get_mut("orchestration") else {
        return;
    };
    let Some(obj) = orch.as_object_mut() else {
        return;
    };
    if !obj.contains_key("hop_failure_policy") {
        obj.insert(
            "hop_failure_policy".into(),
            serde_json::Value::String(policy.to_string()),
        );
        let _ = store.save(&session);
    }
}

pub struct DaemonOrchestrator {
    runtime: Runtime,
    router: AgentRouter,
    config: RmngConfig,
    continuation_locks: SessionContinuationLocks,
}

impl DaemonOrchestrator {
    pub fn new(runtime: Runtime, router: AgentRouter) -> Self {
        Self {
            runtime,
            router,
            config: RmngConfig::load(),
            continuation_locks: SessionContinuationLocks::new(),
        }
    }

    pub fn with_config(runtime: Runtime, router: AgentRouter, config: RmngConfig) -> Self {
        Self {
            runtime,
            router,
            config,
            continuation_locks: SessionContinuationLocks::new(),
        }
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub fn router(&self) -> &AgentRouter {
        &self.router
    }

    /// True when a continuation loop already holds this session's lock.
    pub async fn is_continuation_busy(&self, session_id: &str) -> bool {
        self.continuation_locks.is_busy(session_id).await
    }

    /// Whether post-dispatch background continuation should run.
    pub async fn should_trigger_continue(
        &self,
        session_id: &str,
        intent: &CoreIntent,
        dispatch_resp: &HandleResponse,
    ) -> bool {
        dispatch_resp.ok
            && intent.is_executable()
            && self.should_auto_continue(session_id, self.router.sessions())
            && !self.continuation_locks.is_busy(session_id).await
    }

    /// Run auto-continue for a session until plan.only, failure, or max steps.
    pub async fn continue_session(&self, session_id: &str) -> OrchestrationContinueResponse {
        let _lease = self.continuation_locks.acquire_owned(session_id).await;
        info!(session = session_id, "daemon auto-continue acquired session lock");
        self.run_continue_with_timeout(session_id).await
    }

    async fn run_continue_with_timeout(&self, session_id: &str) -> OrchestrationContinueResponse {
        let timeout = self.config.auto_continue.timeout_secs;
        if timeout > 0 {
            match tokio::time::timeout(
                Duration::from_secs(timeout),
                self.continue_session_inner(session_id),
            )
            .await
            {
                Ok(resp) => resp,
                Err(_) => {
                    warn!(
                        session = session_id,
                        timeout_secs = timeout,
                        "daemon auto-continue timed out; finalizing session"
                    );
                    self.finalize_interrupted(session_id, "timed_out", ContinuationStatus::Failed);
                    OrchestrationContinueResponse::timed_out(session_id, timeout)
                }
            }
        } else {
            self.continue_session_inner(session_id).await
        }
    }

    /// Clear stuck `continuation.status = running` after timeout or external interruption.
    pub fn finalize_interrupted(
        &self,
        session_id: &str,
        orch_status: &str,
        cont_status: ContinuationStatus,
    ) {
        let store = self.router.sessions();
        match store.load(session_id) {
            Ok(mut session) => {
                if let Err(e) = store.finalize_orchestration(&mut session, orch_status, cont_status)
                {
                    warn!(
                        session = session_id,
                        error = %e,
                        "failed to finalize interrupted continuation"
                    );
                } else {
                    info!(
                        session = session_id,
                        orch_status,
                        ?cont_status,
                        "finalized interrupted continuation"
                    );
                }
            }
            Err(e) => {
                warn!(
                    session = session_id,
                    error = %e,
                    "could not load session for interruption cleanup"
                );
            }
        }
    }

    async fn continue_session_inner(&self, session_id: &str) -> OrchestrationContinueResponse {
        let store = self.router.sessions();
        let mut cont = match self.load_or_bootstrap_loop(session_id, store) {
            Ok(c) => c,
            Err(e) => {
                return OrchestrationContinueResponse::failure(session_id, e);
            }
        };

        if cont.step == 0 {
            if let Err(e) = cont.begin_session(store) {
                return OrchestrationContinueResponse::failure(session_id, e.to_string());
            }
        }

        let mut steps_run = 0u32;
        let mut dispatch_actions = Vec::new();
        let max_steps = cont.max_steps;

        for _ in 0..max_steps.saturating_sub(cont.step) {
            let step_result = match cont.run_step(self.router()).await {
                Ok(r) => r,
                Err(e) => {
                    warn!(session = session_id, error = %e, "daemon auto-continue router error");
                    let _ = cont.finish_session(store, "failed", ContinuationStatus::Failed);
                    return OrchestrationContinueResponse {
                        ok: false,
                        action: "orchestration.continue".into(),
                        session_id: session_id.to_string(),
                        steps_run,
                        finished: true,
                        status: "failed".into(),
                        error: Some(e.to_string()),
                        dispatch_actions,
                    };
                }
            };

            match step_result {
                AutoContinueStep::Stop {
                    exit_code: _,
                    reason: AutoContinueStopReason::PlanOnly,
                } => {
                    let _ = cont.finish_session(store, "completed", ContinuationStatus::Done);
                    info!(
                        session = session_id,
                        steps = steps_run,
                        "daemon auto-continue done (plan.only)"
                    );
                    return OrchestrationContinueResponse::success(
                        session_id,
                        steps_run,
                        true,
                        "completed",
                        dispatch_actions,
                    );
                }
                AutoContinueStep::Stop { .. } => {
                    let _ = cont.finish_session(store, "completed", ContinuationStatus::Done);
                    return OrchestrationContinueResponse::success(
                        session_id,
                        steps_run,
                        true,
                        "completed",
                        dispatch_actions,
                    );
                }
                AutoContinueStep::Executed { outcome, intent } => {
                    if outcome.is_handoff() {
                        Self::log_handoff(&outcome);
                    }
                    let resp = self.runtime.handle_core_response(&intent).await;
                    let resp = match resp {
                        Ok(r) => r,
                        Err(e) => {
                            let _ =
                                cont.finish_session(store, "failed", ContinuationStatus::Failed);
                            return OrchestrationContinueResponse::failure(session_id, e.to_string());
                        }
                    };
                    if let Some(action) = &resp.action {
                        dispatch_actions.push(action.clone());
                    }
                    if let Err(e) = persist_dispatch_to_session(store, session_id, &intent, &resp) {
                        warn!(session = session_id, error = %e, "session persist failed");
                    }
                    steps_run += 1;
                    if !resp.ok {
                        let _ = cont.finish_session(store, "failed", ContinuationStatus::Failed);
                        return OrchestrationContinueResponse {
                            ok: false,
                            action: "orchestration.continue".into(),
                            session_id: session_id.to_string(),
                            steps_run,
                            finished: true,
                            status: "failed".into(),
                            error: resp.error.clone(),
                            dispatch_actions,
                        };
                    }
                    if !intent.is_executable() {
                        let _ = cont.finish_session(store, "completed", ContinuationStatus::Done);
                        return OrchestrationContinueResponse::success(
                            session_id,
                            steps_run,
                            true,
                            "completed",
                            dispatch_actions,
                        );
                    }
                    cont.prepare_next_step(&outcome);
                    if let Err(e) = cont.sync_session(store) {
                        warn!(session = session_id, error = %e, "continuation sync failed");
                    }
                    if cont.at_max_steps() {
                        let _ =
                            cont.finish_session(store, "completed", ContinuationStatus::Exhausted);
                        info!(session = session_id, "daemon auto-continue exhausted max steps");
                        return OrchestrationContinueResponse::success(
                            session_id,
                            steps_run,
                            true,
                            "exhausted",
                            dispatch_actions,
                        );
                    }
                }
            }
        }

        let _ = cont.finish_session(store, "completed", ContinuationStatus::Done);
        OrchestrationContinueResponse::success(
            session_id,
            steps_run,
            true,
            "completed",
            dispatch_actions,
        )
    }

    /// After a successful tool dispatch, run continuation if session state requires it.
    pub async fn maybe_continue_after_dispatch(
        &self,
        session_id: &str,
        intent: &CoreIntent,
        dispatch_resp: &HandleResponse,
    ) -> Option<OrchestrationContinueResponse> {
        if !dispatch_resp.ok || !intent.is_executable() {
            return None;
        }
        let store = self.router.sessions();
        if persist_dispatch_to_session(store, session_id, intent, dispatch_resp).is_err() {
            return None;
        }
        if !self.should_auto_continue(session_id, store) {
            return None;
        }
        let _lease = match self.continuation_locks.try_acquire_owned(session_id).await {
            Some(g) => g,
            None => {
                warn!(
                    session = session_id,
                    "daemon background auto-continue skipped (already in progress)"
                );
                return None;
            }
        };
        info!(session = session_id, "daemon post-dispatch auto-continue triggered");
        Some(self.run_continue_with_timeout(session_id).await)
    }

    fn should_auto_continue(&self, session_id: &str, store: &SessionStore) -> bool {
        let Ok(session) = store.load(session_id) else {
            return false;
        };
        if let Some(cont) = SessionStore::chain_continuation(&session) {
            if cont.should_run() {
                return true;
            }
        }
        session
            .shared_context
            .get("orchestration")
            .and_then(|o| o.get("awaiting_continuation"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    fn load_or_bootstrap_loop(
        &self,
        session_id: &str,
        store: &SessionStore,
    ) -> Result<AutoContinueLoop, String> {
        let session = store
            .load(session_id)
            .map_err(|e| format!("session load: {e}"))?;
        if let Some(cont) = SessionStore::chain_continuation(&session) {
            if cont.should_run() {
                return Ok(AutoContinueLoop::from_continuation(session_id, &cont));
            }
        }
        let orch = session
            .shared_context
            .get("orchestration")
            .ok_or_else(|| "no orchestration state on session".to_string())?;
        let awaiting = orch
            .get("awaiting_continuation")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !awaiting {
            return Err("orchestration.continuation not active".into());
        }
        let agent = orch
            .get("continuation_agent")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing continuation_agent".to_string())?;
        let session_max = orch
            .get("continuation")
            .and_then(|c| c.get("max_steps"))
            .and_then(|v| v.as_u64())
            .map(|n| n as u32);
        let max_steps = self.config.auto_continue.resolved_max_steps(session_max);
        apply_default_failure_policy(
            store,
            session_id,
            &self.config.auto_continue.default_failure_policy,
        );
        Ok(AutoContinueLoop::new(
            session_id,
            agent,
            rmng_core::DEFAULT_CONTINUATION_PROMPT,
            max_steps,
        ))
    }

    fn log_handoff(outcome: &RouteOutcome) {
        if let Some(summary) = outcome.chain_outcome_summary() {
            info!(event = "daemon.handoff", summary = %summary);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmng_core::{LlmConfig, LlmProvider};

    fn mock_config() -> RmngConfig {
        RmngConfig {
            llm: LlmConfig {
                llm_provider: LlmProvider::None,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn finalize_interrupted_clears_running_continuation() {
        let dir = std::env::temp_dir().join(format!("rmng-fin-{}", uuid::Uuid::new_v4()));
        let store = SessionStore::new(&dir);
        let session = store.create().expect("create");
        {
            let mut loaded = store.load(&session.id).expect("load");
            store
                .set_orchestration_state(
                    &mut loaded,
                    serde_json::json!({
                        "status": "running",
                        "awaiting_continuation": true,
                        "continuation_agent": "swarm-coordinator",
                        "continuation": {
                            "enabled": true,
                            "max_steps": 3,
                            "step": 1,
                            "start_agent": "swarm-coordinator",
                            "active_agent": "swarm-coordinator",
                            "next_prompt": "go",
                            "status": "running"
                        }
                    }),
                )
                .expect("orch");
        }
        let orch = DaemonOrchestrator::with_config(
            Runtime::bootstrap().unwrap_or_default(),
            AgentRouter::with_session_store(
                rmng_nervous::AgentRegistry::load().expect("registry"),
                rmng_nervous::NervousConnector::from_config(mock_config()),
                store.clone(),
            ),
            mock_config(),
        );
        orch.finalize_interrupted(&session.id, "timed_out", ContinuationStatus::Failed);
        let loaded = store.load(&session.id).expect("load");
        let cont = SessionStore::chain_continuation(&loaded).expect("cont");
        assert!(!cont.should_run());
        assert!(!loaded
            .shared_context
            .get("orchestration")
            .and_then(|o| o.get("awaiting_continuation"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true));
        let _ = std::fs::remove_dir_all(dir);
    }
}