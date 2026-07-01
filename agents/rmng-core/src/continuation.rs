//! Chain continuation state for auto-continue (Sprint 25).
//!
//! Persisted under `shared_context.orchestration.continuation` so a future
//! rmngd worker can resume orchestration without the CLI loop.

use serde::{Deserialize, Serialize};

/// Default follow-up prompt after a successful tool dispatch in auto-continue.
pub const DEFAULT_CONTINUATION_PROMPT: &str = "Continue the orchestration. If specialist work is complete, emit plan.only with handoff_return_to swarm-coordinator summarizing recent_tool_results. Otherwise execute the next required tool. Do not repeat successful tools.";

/// Lifecycle of an auto-continue loop (CLI today; daemon tomorrow).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuationStatus {
    Pending,
    Running,
    Done,
    Exhausted,
    Failed,
}

/// Serializable continuation cursor stored on the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainContinuation {
    pub enabled: bool,
    pub max_steps: u32,
    pub step: u32,
    pub start_agent: String,
    pub active_agent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_prompt: Option<String>,
    pub next_prompt: String,
    pub status: ContinuationStatus,
}

impl ChainContinuation {
    pub fn new(start_agent: &str, initial_prompt: &str, max_steps: u32) -> Self {
        Self {
            enabled: true,
            max_steps,
            step: 0,
            start_agent: start_agent.to_string(),
            active_agent: start_agent.to_string(),
            initial_prompt: Some(initial_prompt.to_string()),
            next_prompt: DEFAULT_CONTINUATION_PROMPT.to_string(),
            status: ContinuationStatus::Pending,
        }
    }

    pub fn should_run(&self) -> bool {
        self.enabled
            && matches!(
                self.status,
                ContinuationStatus::Pending | ContinuationStatus::Running
            )
            && self.step < self.max_steps
    }

    pub fn mark_running(&mut self) {
        self.status = ContinuationStatus::Running;
    }

    pub fn advance_after_dispatch(&mut self, next_agent: &str) {
        self.step += 1;
        self.active_agent = next_agent.to_string();
        self.step_prompt_assign(DEFAULT_CONTINUATION_PROMPT);
    }

    pub fn step_prompt_assign(&mut self, prompt: &str) {
        self.next_prompt = prompt.to_string();
    }

    pub fn current_prompt(&self) -> &str {
        if self.step == 0 {
            self.initial_prompt
                .as_deref()
                .unwrap_or(DEFAULT_CONTINUATION_PROMPT)
        } else {
            &self.next_prompt
        }
    }

    pub fn at_max_steps(&self) -> bool {
        self.step + 1 >= self.max_steps
    }

    pub fn finish(&mut self, status: ContinuationStatus) {
        self.enabled = false;
        self.status = status;
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::json!({}))
    }

    pub fn from_value(value: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(value.clone()).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuation_advances_steps() {
        let mut c = ChainContinuation::new("swarm-coordinator", "start task", 3);
        assert_eq!(c.current_prompt(), "start task");
        c.mark_running();
        c.advance_after_dispatch("repo-keeper");
        assert_eq!(c.step, 1);
        assert_eq!(c.active_agent, "repo-keeper");
        assert!(c.should_run());
    }

    #[test]
    fn continuation_stops_at_max() {
        let mut c = ChainContinuation::new("a", "go", 1);
        c.mark_running();
        assert!(c.at_max_steps());
        c.finish(ContinuationStatus::Exhausted);
        assert!(!c.should_run());
    }
}
