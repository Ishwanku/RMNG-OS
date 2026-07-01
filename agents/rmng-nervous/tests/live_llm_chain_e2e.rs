//! Live LLM chain tests — skip without API keys / Ollama.

use rmng_core::{LlmConfig, LlmProvider, RmngConfig, SessionStore};
use rmng_nervous::{AgentRouter, NervousConnector, RouteOutcome};

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

async fn run_chain_ask(cfg: RmngConfig, label: &str) {
    let dir = std::env::temp_dir().join(format!("rmng-live-chain-{label}-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let registry = rmng_nervous::AgentRegistry::load().expect("registry");
    let router = AgentRouter::with_session_store(registry, NervousConnector::from_config(cfg), store);

    let prompt = r#"Return ONE raw JSON object (plan.only). metadata.handoff_chain MUST be JSON array [\"swarm-coordinator\",\"repo-keeper\"] NOT comma string. Include metadata.session_id and metadata.chain_id."#;
    let outcome = router
        .ask_routed(Some(&session.id), "swarm-coordinator", prompt)
        .await
        .expect("ask");

    eprintln!("[{label}] outcome: handoff={}", outcome.is_handoff());
    if let RouteOutcome::HandoffChain { chain, .. } = &outcome {
        assert!(chain.len() >= 2, "expected chain from live LLM");
    } else if let RouteOutcome::Handoff { to_agent, .. } = &outcome {
        assert!(!to_agent.is_empty());
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn live_groq_handoff_chain_or_delegate() {
    let Some(cfg) = groq_config() else {
        eprintln!("skip: GROQ_API_KEY not set");
        return;
    };
    run_chain_ask(cfg, "groq").await;
}

#[tokio::test]
async fn live_grok_handoff_chain_or_delegate() {
    let Some(cfg) = grok_config() else {
        eprintln!("skip: XAI_API_KEY not set");
        return;
    };
    run_chain_ask(cfg, "grok").await;
}
