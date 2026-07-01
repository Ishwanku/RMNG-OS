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
    fn options_from_metadata() {
        let mut meta = base_meta();
        meta.hop_failure_policy = Some("skip".into());
        meta.hop_retry_max = Some(5);
        let opts = HandoffChainOptions::from_metadata(&meta);
        assert_eq!(opts.hop_failure_policy, HopFailurePolicy::Skip);
        assert_eq!(opts.hop_retry_max, 5);
    }
}
