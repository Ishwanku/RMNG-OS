//! Live LLM multi-hop chain emission (Sprint 25–30).
//! Skip without API keys / Ollama. Set RMNG_CHAIN_STRICT=0 to allow single handoff_to fallback.

use rmng_core::{LlmConfig, LlmProvider, RmngConfig, SessionStore};
use rmng_nervous::{AgentRouter, NervousConnector, RouteOutcome};

fn chain_strict() -> bool {
    std::env::var("RMNG_CHAIN_STRICT")
        .map(|v| v != "0" && v != "false")
        .unwrap_or(true)
}

fn groq_config() -> Option<RmngConfig> {
    if std::env::var("GROQ_API_KEY").is_err() {
        return None;
    }
    Some(RmngConfig {
        llm: LlmConfig {
            llm_provider: LlmProvider::Groq,
            model: Some("llama-3.3-70b-versatile".into()),
            ..Default::default()
        },
        profile: Some("groq-fast".into()),
        profiles: vec![],
        ..Default::default()
    })
}

fn grok_config() -> Option<RmngConfig> {
    if std::env::var("XAI_API_KEY").is_err() {
        return None;
    }
    Some(RmngConfig {
        llm: LlmConfig {
            llm_provider: LlmProvider::Grok,
            model: Some("grok-3-mini".into()),
            ..Default::default()
        },
        profile: Some("grok-frontier".into()),
        profiles: vec![],
        ..Default::default()
    })
}

async fn ollama_config() -> Option<RmngConfig> {
    let ok = rmng_nervous::OllamaProvider::default()
        .health()
        .await
        .unwrap_or(false);
    if !ok {
        return None;
    }
    Some(RmngConfig {
        llm: LlmConfig {
            llm_provider: LlmProvider::Ollama,
            model: Some("llama3.2".into()),
            endpoint_url: Some("http://127.0.0.1:11434".into()),
            ..Default::default()
        },
        ..Default::default()
    })
}

fn git_chain_prompt(session_id: &str) -> String {
    format!(
        r#"User request: Coordinate a git hygiene workflow across multiple agents.

You are swarm-coordinator (L4). Emit exactly ONE plan.only JSON object.

REQUIRED:
- metadata.session_id = "{session_id}"
- metadata.chain_id = "{session_id}"
- metadata.handoff_chain = ["swarm-coordinator","repo-keeper","runtime-executor"] as a JSON array (NOT a comma string)

Do NOT use markdown fences. Do NOT use handoff_to when a chain is needed."#
    )
}

fn assert_chain_emission(outcome: &RouteOutcome, label: &str) {
    match outcome {
        RouteOutcome::HandoffChain { chain, .. } => {
            assert!(
                chain.len() >= 2,
                "[{label}] handoff_chain too short: {chain:?}"
            );
            assert_eq!(
                chain[0], "swarm-coordinator",
                "[{label}] chain must start with swarm-coordinator"
            );
            eprintln!("[{label}] OK HandoffChain: {chain:?}");
        }
        RouteOutcome::Handoff { to_agent, .. } => {
            eprintln!("[{label}] WARN single Handoff to {to_agent} (expected HandoffChain)");
            if chain_strict() {
                panic!("[{label}] expected HandoffChain with len >= 2; set RMNG_CHAIN_STRICT=0 to allow fallback");
            }
            assert!(!to_agent.is_empty());
        }
        other => {
            panic!("[{label}] unexpected outcome: {other:?}");
        }
    }
}

async fn run_chain_emission_test(cfg: RmngConfig, label: &str) {
    let dir = std::env::temp_dir().join(format!("rmng-live-chain-{label}-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let registry = rmng_nervous::AgentRegistry::load().expect("registry");
    let router = AgentRouter::with_session_store(registry, NervousConnector::from_config(cfg), store);

    let prompt = git_chain_prompt(&session.id);
    let outcome = router
        .ask_routed(Some(&session.id), "swarm-coordinator", &prompt)
        .await
        .expect("ask");

    assert_chain_emission(&outcome, label);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn live_groq_emits_handoff_chain() {
    let Some(cfg) = groq_config() else {
        eprintln!("skip: GROQ_API_KEY not set");
        return;
    };
    run_chain_emission_test(cfg, "groq").await;
}

#[tokio::test]
async fn live_grok_emits_handoff_chain() {
    let Some(cfg) = grok_config() else {
        eprintln!("skip: XAI_API_KEY not set");
        return;
    };
    run_chain_emission_test(cfg, "grok").await;
}

#[tokio::test]
async fn live_ollama_emits_handoff_chain() {
    let Some(cfg) = ollama_config().await else {
        eprintln!("skip: Ollama not reachable at 127.0.0.1:11434");
        return;
    };
    run_chain_emission_test(cfg, "ollama").await;
}