//! Live LLM handoff_return_to E2E (Sprint 31) — specialist returns to orchestrator.

#[path = "live_llm_live_helpers.rs"]
mod helpers;

use helpers::{
    anthropic_config, assert_return_to_orchestrator, google_config, grok_config, groq_config,
    ollama_config, openai_config, return_to_prompt, seed_repo_keeper_tool_result,
};

async fn run_return_to_test(cfg: rmng_core::RmngConfig, label: &str) {
    let (router, store, session_id, dir) = helpers::router_with_config(cfg, label).await;
    seed_repo_keeper_tool_result(&store, &session_id);
    let prompt = return_to_prompt(&session_id);
    let outcome = router
        .ask_routed(Some(&session_id), "repo-keeper", &prompt)
        .await
        .expect("ask");
    assert_return_to_orchestrator(&outcome, label);
    let loaded = store.load(&session_id).expect("load");
    assert!(
        loaded.handoff_history.iter().any(|h| h.to_agent == "swarm-coordinator"),
        "should record return hop"
    );
    let _ = std::fs::remove_dir_all(dir);
}

macro_rules! live_return_test {
    ($name:ident, $cfg_fn:ident, $skip:expr) => {
        #[tokio::test]
        async fn $name() {
            let Some(cfg) = $cfg_fn() else {
                eprintln!("skip: {}", $skip);
                return;
            };
            run_return_to_test(cfg, stringify!($name)).await;
        }
    };
    ($name:ident, async $cfg_fn:ident, $skip:expr) => {
        #[tokio::test]
        async fn $name() {
            let Some(cfg) = $cfg_fn().await else {
                eprintln!("skip: {}", $skip);
                return;
            };
            run_return_to_test(cfg, stringify!($name)).await;
        }
    };
}

live_return_test!(live_groq_handoff_return_to, groq_config, "GROQ_API_KEY not set");
live_return_test!(live_grok_handoff_return_to, grok_config, "XAI_API_KEY not set");
live_return_test!(live_openai_handoff_return_to, openai_config, "OPENAI_API_KEY not set");
live_return_test!(live_anthropic_handoff_return_to, anthropic_config, "ANTHROPIC_API_KEY not set");
live_return_test!(live_google_handoff_return_to, google_config, "GOOGLE_API_KEY not set");
live_return_test!(live_ollama_handoff_return_to, async ollama_config, "Ollama not reachable");