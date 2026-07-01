//! Multi-hop orchestration policy types (Sprint 25).

use crate::intent::Metadata;
use serde::{Deserialize, Serialize};

/// Action when an individual chain hop fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HopFailurePolicy {
    /// Stop the chain and surface the error (default, backward compatible).
    #[default]
    Abort,
    /// Retry the same hop up to `hop_retry_max` times, then abort.
    Retry,
    /// Record the failure, skip the failed target, and attempt a shortcut to the next agent.
    Skip,
}

/// Options controlling multi-hop chain execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffChainOptions {
    pub hop_failure_policy: HopFailurePolicy,
    /// Used only when policy is `Retry`. Default: 2.
    pub hop_retry_max: u32,
}

impl Default for HandoffChainOptions {
    fn default() -> Self {
        Self {
            hop_failure_policy: HopFailurePolicy::Abort,
            hop_retry_max: 2,
        }
    }
}

impl HandoffChainOptions {
    pub const DEFAULT_RETRY_MAX: u32 = 2;

    pub fn from_metadata(meta: &Metadata) -> Self {
        let hop_failure_policy = meta
            .hop_failure_policy
            .as_deref()
            .and_then(parse_hop_failure_policy)
            .unwrap_or_default();
        let hop_retry_max = meta
            .hop_retry_max
            .unwrap_or(Self::DEFAULT_RETRY_MAX)
            .max(1);
        Self {
            hop_failure_policy,
            hop_retry_max,
        }
    }
}

/// One recorded hop error surfaced to the orchestrator (Sprint 25).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainHopError {
    pub hop_index: usize,
    pub from_agent: String,
    pub to_agent: String,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

/// Parsed view of `shared_context.orchestration` for recovery and CLI feedback.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OrchestrationSnapshot {
    pub status: Option<String>,
    pub chain_id: Option<String>,
    pub failed_hop: Option<usize>,
    pub failed_from: Option<String>,
    pub failed_to: Option<String>,
    pub last_error: Option<String>,
    pub hop_errors: Vec<ChainHopError>,
    pub skipped_hops: Vec<serde_json::Value>,
    pub hop_decisions: Vec<serde_json::Value>,
}

impl OrchestrationSnapshot {
    pub fn from_value(value: &serde_json::Value) -> Self {
        let status = value.get("status").and_then(|v| v.as_str()).map(str::to_string);
        let chain_id = value.get("chain_id").and_then(|v| v.as_str()).map(str::to_string);
        let failed_hop = value.get("failed_hop").and_then(|v| v.as_u64()).map(|n| n as usize);
        let failed_from = value.get("failed_from").and_then(|v| v.as_str()).map(str::to_string);
        let failed_to = value.get("failed_to").and_then(|v| v.as_str()).map(str::to_string);
        let last_error = value
            .get("error")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let hop_errors = value
            .get("hop_errors")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| serde_json::from_value(e.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();
        let skipped_hops = value
            .get("skipped_hops")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let hop_decisions = value
            .get("hop_decisions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Self {
            status,
            chain_id,
            failed_hop,
            failed_from,
            failed_to,
            last_error,
            hop_errors,
            skipped_hops,
            hop_decisions,
        }
    }

    pub fn has_failures(&self) -> bool {
        self.status.as_deref() == Some("failed")
            || !self.hop_errors.is_empty()
            || !self.skipped_hops.is_empty()
    }

    /// LLM-facing recovery block injected into orchestrator prompts.
    pub fn recovery_hints(&self) -> String {
        if !self.has_failures() && self.status.as_deref() != Some("completed_with_skips") {
            return String::new();
        }
        let mut lines = vec!["## Chain error recovery (Sprint 25)".to_string()];
        if let Some(status) = &self.status {
            lines.push(format!("status: {status}"));
        }
        if let (Some(hop), Some(from), Some(to), Some(err)) = (
            self.failed_hop,
            &self.failed_from,
            &self.failed_to,
            &self.last_error,
        ) {
            lines.push(format!(
                "terminal_failure: hop {hop} {from} → {to}: {err}"
            ));
        }
        if !self.hop_errors.is_empty() {
            lines.push("hop_errors:".into());
            for e in &self.hop_errors {
                let action = e
                    .action
                    .as_deref()
                    .map(|a| format!(" [{a}]"))
                    .unwrap_or_default();
                lines.push(format!(
                    "  - hop {}: {} → {}{}: {}",
                    e.hop_index, e.from_agent, e.to_agent, action, e.error
                ));
            }
        }
        if !self.skipped_hops.is_empty() {
            lines.push("skipped_hops:".into());
            for s in &self.skipped_hops {
                let hop = s.get("hop_index").and_then(|v| v.as_u64()).unwrap_or(0);
                let from = s.get("from_agent").and_then(|v| v.as_str()).unwrap_or("?");
                let skipped = s.get("skipped_agent").and_then(|v| v.as_str()).unwrap_or("?");
                let err = s.get("error").and_then(|v| v.as_str()).unwrap_or("?");
                lines.push(format!(
                    "  - hop {hop}: {from} skipped {skipped}: {err}"
                ));
            }
        }
        lines.push(
            "Recovery options: emit a new plan.only with an alternate handoff_chain or handoff_to;              adjust hop_failure_policy; or synthesize from recent_tool_results if partial work succeeded."
                .into(),
        );
        lines.join("
")
    }

    /// Human-readable summary for CLI stderr on chain failure.
    pub fn cli_failure_report(&self) -> String {
        let mut lines = Vec::new();
        if let Some(status) = &self.status {
            lines.push(format!("orchestration status: {status}"));
        }
        if let (Some(hop), Some(from), Some(to)) = (self.failed_hop, &self.failed_from, &self.failed_to)
        {
            lines.push(format!("failed hop {hop}: {from} → {to}"));
        }
        if let Some(err) = &self.last_error {
            lines.push(format!("error: {err}"));
        }
        for e in &self.hop_errors {
            lines.push(format!(
                "  hop {}: {} → {} — {}",
                e.hop_index, e.from_agent, e.to_agent, e.error
            ));
        }
        for s in &self.skipped_hops {
            let hop = s.get("hop_index").and_then(|v| v.as_u64()).unwrap_or(0);
            let skipped = s.get("skipped_agent").and_then(|v| v.as_str()).unwrap_or("?");
            let err = s.get("error").and_then(|v| v.as_str()).unwrap_or("?");
            lines.push(format!("  skipped hop {hop} ({skipped}): {err}"));
        }
        if let Some(decisions) = self.hop_decisions.last() {
            if let Some(action) = decisions.get("action").and_then(|v| v.as_str()) {
                lines.push(format!("last policy action: {action}"));
            }
        }
        lines.join("
")
    }
}

/// Parse policy strings from LLM metadata or CLI (`retry` / `skip` / `abort`).
pub fn parse_hop_failure_policy(raw: &str) -> Option<HopFailurePolicy> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "abort" | "" => Some(HopFailurePolicy::Abort),
        "retry" => Some(HopFailurePolicy::Retry),
        "skip" => Some(HopFailurePolicy::Skip),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::Metadata;

    fn base_meta() -> Metadata {
        Metadata {
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
        }
    }

    #[test]
    fn parses_hop_failure_policies() {
        assert_eq!(
            parse_hop_failure_policy("retry"),
            Some(HopFailurePolicy::Retry)
        );
        assert_eq!(parse_hop_failure_policy("SKIP"), Some(HopFailurePolicy::Skip));
        assert_eq!(parse_hop_failure_policy("abort"), Some(HopFailurePolicy::Abort));
        assert!(parse_hop_failure_policy("unknown").is_none());
    }

    #[test]
    fn options_default_is_abort() {
        let opts = HandoffChainOptions::from_metadata(&base_meta());
        assert_eq!(opts.hop_failure_policy, HopFailurePolicy::Abort);
        assert_eq!(opts.hop_retry_max, 2);
    }

    #[test]
    fn snapshot_formats_recovery_hints() {
        let value = serde_json::json!({
            "status": "failed",
            "failed_hop": 0,
            "failed_from": "swarm-coordinator",
            "failed_to": "repo-keeper",
            "error": "policy denied",
            "hop_errors": [{
                "hop_index": 0,
                "from_agent": "swarm-coordinator",
                "to_agent": "repo-keeper",
                "error": "policy denied",
                "action": "abort"
            }]
        });
        let snap = OrchestrationSnapshot::from_value(&value);
        let hints = snap.recovery_hints();
        assert!(hints.contains("terminal_failure"));
        assert!(hints.contains("policy denied"));
        assert!(snap.cli_failure_report().contains("failed hop 0"));
    }

    #[test]
    fn options_from_metadata() {
        let mut meta = base_meta();
        meta.hop_failure_policy = Some("skip".into());
        meta.hop_retry_max = Some(5);
        let opts = HandoffChainOptions::from_metadata(&meta);
        assert_eq!(opts.hop_failure_policy, HopFailurePolicy::Skip);
        assert_eq!(opts.hop_retry_max, 5);
    }
}
