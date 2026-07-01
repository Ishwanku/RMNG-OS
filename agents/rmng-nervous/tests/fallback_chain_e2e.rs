//! Fallback chain integration tests (Sprint 9) — simulates provider failures without live APIs.

use rmng_core::{LlmProvider, RmngConfig};
use rmng_nervous::chain::run_fallback_chain;
use rmng_nervous::providers::ProviderError;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[tokio::test]
async fn fallback_chain_traverses_on_rate_limit_and_model_not_found() {
    let calls = Arc::new(AtomicUsize::new(0));
    let calls2 = calls.clone();
    let result = run_fallback_chain(
        3,
        move |idx| {
            let calls = calls2.clone();
            async move {
                let n = calls.fetch_add(1, Ordering::SeqCst);
                match n {
                    0 => Err(ProviderError::api("groq", 429, "rate limit exceeded")),
                    1 => Err(ProviderError::api("grok", 404, "model not found")),
                    _ => Ok(format!("success-at-{idx}")),
                }
            }
        },
        |e: &ProviderError| e.warrants_provider_fallback(),
    )
    .await
    .expect("chain should succeed on third provider");

    assert_eq!(result.attempt_index, 2);
    assert_eq!(result.prior_failures.len(), 2);
    assert!(result.prior_failures[0].contains("rate limit"));
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn fallback_chain_stops_on_invalid_key() {
    let result = run_fallback_chain(
        2,
        |idx| async move {
            if idx == 0 {
                Err(ProviderError::api("openai", 401, "invalid api key"))
            } else {
                Ok("should not reach".to_string())
            }
        },
        |e: &ProviderError| e.warrants_provider_fallback(),
    )
    .await;
    assert!(result.is_err());
}

#[test]
fn resolved_chain_preserves_per_agent_fallback_order() {
    let raw = r#"
llm_fallback = ["grok-frontier"]

[llm]
llm_provider = "groq"
model = "llama-3.3-70b-versatile"

[[profiles]]
name = "grok-frontier"
llm_provider = "grok"
model = "grok-4.3"
"#;
    let cfg: RmngConfig = toml::from_str(raw).unwrap();
    let chain = cfg.resolved_llm_chain_for_agent(None);
    assert_eq!(chain.len(), 2);
    assert_eq!(chain[1].label, "fallback:grok-frontier");
    assert_eq!(chain[1].config.llm_provider, LlmProvider::Grok);
}