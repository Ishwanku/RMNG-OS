//! Auto-continue loop extracted from CLI for daemon reuse (Sprint 25).

use crate::router::{AgentRouter, RouteOutcome, RouterError};
use rmng_core::session::SessionStore;
use rmng_core::{
    ChainContinuation, ContinuationStatus, CoreIntent, SessionError, DEFAULT_CONTINUATION_PROMPT,
};

/// Why an auto-continue loop stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoContinueStopReason {
    PlanOnly,
    DispatchFailed,
    RouterError,
    MaxSteps,
}

/// Result of one auto-continue iteration.
#[derive(Debug, Clone)]
pub enum AutoContinueStep {
    Stop {
        exit_code: i32,
        reason: AutoContinueStopReason,
    },
    Executed {
        outcome: RouteOutcome,
        intent: CoreIntent,
    },
}

/// Session-backed auto-continue driver (CLI and future rmngd worker).
#[derive(Debug, Clone)]
pub struct AutoContinueLoop {
    pub session_id: String,
    pub agent_id: String,
    pub step: u32,
    pub max_steps: u32,
    pub initial_prompt: String,
    pub step_prompt: String,
}

impl AutoContinueLoop {
    pub fn new(
        session_id: &str,
        start_agent: &str,
        prompt: &str,
        max_steps: u32,
    ) -> Self {
        Self {
            session_id: session_id.to_string(),
            agent_id: start_agent.to_string(),
            step: 0,
            max_steps,
            initial_prompt: prompt.to_string(),
            step_prompt: prompt.to_string(),
        }
    }

    pub fn from_continuation(session_id: &str, cont: &ChainContinuation) -> Self {
        Self {
            session_id: session_id.to_string(),
            agent_id: cont.active_agent.clone(),
            step: cont.step,
            max_steps: cont.max_steps,
            initial_prompt: cont
                .initial_prompt
                .clone()
                .unwrap_or_else(|| DEFAULT_CONTINUATION_PROMPT.to_string()),
            step_prompt: cont.current_prompt().to_string(),
        }
    }

    pub fn to_continuation(&self) -> ChainContinuation {
        let mut c = ChainContinuation::new(&self.agent_id, &self.initial_prompt, self.max_steps);
        c.step = self.step;
        c.active_agent = self.agent_id.clone();
        c.next_prompt = self.step_prompt.clone();
        c.status = if self.step == 0 {
            ContinuationStatus::Pending
        } else {
            ContinuationStatus::Running
        };
        c
    }

    /// Persist continuation cursor before the first step (daemon-resumable).
    pub fn begin_session(&self, store: &SessionStore) -> Result<(), SessionError> {
        let mut session = store.load(&self.session_id)?;
        let mut cont = self.to_continuation();
        cont.mark_running();
        store.set_chain_continuation(&mut session, &cont)
    }

    pub fn sync_session(&self, store: &SessionStore) -> Result<(), SessionError> {
        let mut session = store.load(&self.session_id)?;
        let mut cont = self.to_continuation();
        cont.mark_running();
        store.set_chain_continuation(&mut session, &cont)
    }

    pub fn finish_session(
        &self,
        store: &SessionStore,
        orch_status: &str,
        cont_status: ContinuationStatus,
    ) -> Result<(), SessionError> {
        let mut session = store.load(&self.session_id)?;
        store.finalize_orchestration(&mut session, orch_status, cont_status)
    }

    pub fn current_prompt(&self) -> &str {
        if self.step == 0 {
            &self.initial_prompt
        } else {
            &self.step_prompt
        }
    }

    pub async fn run_step(&mut self, router: &AgentRouter) -> Result<AutoContinueStep, RouterError> {
        let outcome = router
            .ask_routed(
                Some(&self.session_id),
                &self.agent_id,
                self.current_prompt(),
            )
            .await?;
        let mut intent = outcome.intent();
        let handoff_from = outcome.handoff_from_agent();
        AgentRouter::enrich_intent_metadata(
            &mut intent,
            Some(&self.session_id),
            handoff_from,
        );
        if !intent.is_executable() {
            return Ok(AutoContinueStep::Stop {
                exit_code: 0,
                reason: AutoContinueStopReason::PlanOnly,
            });
        }
        Ok(AutoContinueStep::Executed { outcome, intent })
    }

    pub fn prepare_next_step(&mut self, outcome: &RouteOutcome) {
        if let Some(next) = outcome.final_agent() {
            self.agent_id = next.to_string();
        }
        self.step += 1;
        self.step_prompt = DEFAULT_CONTINUATION_PROMPT.to_string();
    }

    pub fn at_max_steps(&self) -> bool {
        self.step + 1 >= self.max_steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_roundtrip_continuation() {
        let l = AutoContinueLoop::new("sid", "swarm-coordinator", "task", 3);
        let c = l.to_continuation();
        let l2 = AutoContinueLoop::from_continuation("sid", &c);
        assert_eq!(l2.agent_id, "swarm-coordinator");
        assert_eq!(l2.max_steps, 3);
    }
}
