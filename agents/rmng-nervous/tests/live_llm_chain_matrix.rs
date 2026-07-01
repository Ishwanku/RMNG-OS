//! Sprint 30 — per-provider chain emission matrix (live API, run with --ignored).
//!
//! ```bash
//! cargo test -p rmng-nervous --test live_llm_chain_matrix -- --ignored --nocapture
//! ```

use rmng_core::{LlmConfig, LlmProvider, RmngConfig, SessionStore};
use rmng_nervous::{parse_core_intent, AgentRouter, NervousConnector};

fn git_chain_prompt(session_id: &str) -> String {
    format!(
        r#"Emit plan.only with metadata.handoff_chain as JSON array \
["swarm-coordinator","repo-keeper","runtime-executor"]. session_id="{session_id}"."#
    )
}

async fn emission_matrix_row(provider: LlmProvider, model: &str, label: &str) -> bool {
    let cfg = RmngConfig {
        llm: LlmConfig {
            llm_provider: provider,
            model: Some(model.into()),
            ..Default::default()
        },
        ..Default::default()
    };
    let dir = std::env::temp_dir().join(format!("rmng-matrix-{label}-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let registry = rmng_nervous::AgentRegistry::load().expect("registry");
    let router = AgentRouter::with_session_store(registry, NervousConnector::from_config(cfg), store);
    let prompt = git_chain_prompt(&session.id);
    let outcome = router
        .ask_routed(Some(&session.id), "swarm-coordinator", &prompt)
        .await;
    let _ = std::fs::remove_dir_all(dir);
    match outcome {
        Ok(o) if o.is_handoff() => {
            eprintln!("[matrix {label}] handoff ok");
            true
        }
        Ok(o) => {
            eprintln!("[matrix {label}] no handoff: {o:?}");
            false
        }
        Err(e) => {
            eprintln!("[matrix {label}] error: {e}");
            false
        }
    }
}

#[test]
fn parse_matrix_normalization_smoke() {
    let samples = [
        r#"{"action":"plan.only","reasoning":"x","metadata":{"handoff_chain":"a,b"}}"#,
        r#"{"action":"plan.only","reasoning":"x","metadata":{"handoff_chain":"a;b"}}"#,
        r#"{"action":"plan.only","reasoning":"x","metadata":{"handoff_chain":["a","b"]}}"#,
    ];
    for raw in samples {
        let intent = parse_core_intent(raw).expect("parse");
        assert!(intent.metadata().unwrap().handoff_chain.is_some());
    }
}

#[tokio::test]
#[ignore = "live GROQ_API_KEY — chain emission matrix"]
async fn matrix_groq_chain_emission() {
    if std::env::var("GROQ_API_KEY").is_err() {
        return;
    }
    assert!(emission_matrix_row(LlmProvider::Groq, "llama-3.3-70b-versatile", "groq").await);
}

#[tokio::test]
#[ignore = "live XAI_API_KEY — chain emission matrix"]
async fn matrix_grok_chain_emission() {
    if std::env::var("XAI_API_KEY").is_err() {
        return;
    }
    assert!(emission_matrix_row(LlmProvider::Grok, "grok-3-mini", "grok").await);
}