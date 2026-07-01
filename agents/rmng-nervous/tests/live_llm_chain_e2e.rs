//! Live LLM multi-hop chain emission (Sprint 25–31).

#[path = "live_llm_live_helpers.rs"]
mod helpers;

use helpers::{
    anthropic_config, assert_handoff_chain, git_chain_prompt, google_config, grok_config,
    groq_config, ollama_config, openai_config,
};
async fn run_chain_emission_test(
    cfg: rmng_core::RmngConfig,
    label: &str,
) {
    let (router, store, session_id, dir) = helpers::router_with_config(cfg, label).await;
    let prompt = git_chain_prompt(&session_id);
    let outcome = router
        .ask_routed(Some(&session_id), "swarm-coordinator", &prompt)
        .await
        .expect("ask");
    assert_handoff_chain(&outcome, label);
    let _ = store;
    let _ = std::fs::remove_dir_all(dir);
}

macro_rules! live_chain_test {
    ($name:ident, $cfg_fn:ident, $skip:expr) => {
        #[tokio::test]
        async fn $name() {
            let Some(cfg) = $cfg_fn() else {
                eprintln!("skip: {}", $skip);
                return;
            };
            run_chain_emission_test(cfg, stringify!($name)).await;
        }
    };
    ($name:ident, async $cfg_fn:ident, $skip:expr) => {
        #[tokio::test]
        async fn $name() {
            let Some(cfg) = $cfg_fn().await else {
                eprintln!("skip: {}", $skip);
                return;
            };
            run_chain_emission_test(cfg, stringify!($name)).await;
        }
    };
}

live_chain_test!(live_groq_emits_handoff_chain, groq_config, "GROQ_API_KEY not set");
live_chain_test!(live_grok_emits_handoff_chain, grok_config, "XAI_API_KEY not set");
live_chain_test!(live_openai_emits_handoff_chain, openai_config, "OPENAI_API_KEY not set");
live_chain_test!(live_anthropic_emits_handoff_chain, anthropic_config, "ANTHROPIC_API_KEY not set");
live_chain_test!(live_google_emits_handoff_chain, google_config, "GOOGLE_API_KEY not set");
live_chain_test!(live_ollama_emits_handoff_chain, async ollama_config, "Ollama not reachable");