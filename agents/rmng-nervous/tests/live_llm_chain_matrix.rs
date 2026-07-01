//! Sprint 30–31 — per-provider chain emission matrix (live API, run with --ignored).

#[path = "live_llm_live_helpers.rs"]
mod helpers;

use helpers::git_chain_prompt;
use rmng_core::{LlmConfig, LlmProvider, RmngConfig};
use rmng_nervous::{parse_core_intent, RouteOutcome};

async fn emission_matrix_row(provider: LlmProvider, model: &str, label: &str) -> bool {
    let cfg = RmngConfig {
        llm: LlmConfig {
            llm_provider: provider,
            model: Some(model.into()),
            ..Default::default()
        },
        ..Default::default()
    };
    let (router, _store, session_id, dir) = helpers::router_with_config(cfg, label).await;
    let prompt = git_chain_prompt(&session_id);
    let result = router
        .ask_routed(Some(&session_id), "swarm-coordinator", &prompt)
        .await;
    let _ = std::fs::remove_dir_all(dir);
    match result {
        Ok(outcome) => match &outcome {
            RouteOutcome::HandoffChain { chain, .. }
                if chain.len() >= 2 && chain[0] == "swarm-coordinator" =>
            {
                eprintln!("[matrix {label}] OK HandoffChain: {chain:?}");
                true
            }
            other => {
                eprintln!("[matrix {label}] strict chain miss: {other:?}");
                false
            }
        },
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

#[tokio::test]
#[ignore = "live OPENAI_API_KEY — chain emission matrix"]
async fn matrix_openai_chain_emission() {
    if std::env::var("OPENAI_API_KEY").is_err() {
        return;
    }
    assert!(emission_matrix_row(LlmProvider::OpenAi, "gpt-4o-mini", "openai").await);
}

#[tokio::test]
#[ignore = "live ANTHROPIC_API_KEY — chain emission matrix"]
async fn matrix_anthropic_chain_emission() {
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        return;
    }
    assert!(
        emission_matrix_row(
            LlmProvider::Anthropic,
            "claude-3-5-haiku-20241022",
            "anthropic"
        )
        .await
    );
}

#[tokio::test]
#[ignore = "live GOOGLE_API_KEY — chain emission matrix"]
async fn matrix_google_chain_emission() {
    if std::env::var("GOOGLE_API_KEY").is_err() {
        return;
    }
    assert!(emission_matrix_row(LlmProvider::Google, "gemini-2.0-flash", "google").await);
}