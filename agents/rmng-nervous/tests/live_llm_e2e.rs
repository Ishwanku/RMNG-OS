//! Live LLM orchestration tests — skip when Ollama is unavailable.

use rmng_core::{LlmConfig, LlmProvider, RmngConfig, SessionStore, ToolResultRecord};
use rmng_nervous::{AgentRouter, NervousConnector, OllamaProvider, RouteOutcome};
use std::path::PathBuf;

fn ollama_config() -> RmngConfig {
    RmngConfig {
        llm: LlmConfig {
            llm_provider: LlmProvider::Ollama,
            endpoint_url: Some("http://127.0.0.1:11434".into()),
            api_key_env_var: None,
            model: Some(
                std::env::var("RMNG_OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".into()),
            ),
            ..Default::default()
        },
        profile: None,
        profiles: vec![],
    }
}

async fn ollama_available() -> bool {
    OllamaProvider::default().health().await.unwrap_or(false)
}

fn test_router_with_ollama(store: SessionStore) -> AgentRouter {
    let registry = rmng_nervous::AgentRegistry::load().expect("registry");
    let connector = NervousConnector::from_config(ollama_config());
    AgentRouter::with_session_store(registry, connector, store)
}

#[tokio::test]
async fn live_llm_assembled_prompt_includes_session_orchestration_guide() {
    use rmng_nervous::skill::assemble_prompt_full;
    let dir = std::env::temp_dir().join(format!("rmng-live-prompt-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let mut session = store.create().expect("create");
    store
        .record_tool_result(
            &mut session,
            ToolResultRecord {
                timestamp: chrono::Utc::now(),
                tool: "git.status".into(),
                parameters: serde_json::json!({}),
                output: "On branch main".into(),
                success: true,
                exit_code: Some(0),
                handoff_from: None,
            },
        )
        .expect("record");
    let reg = rmng_nervous::AgentRegistry::load_from(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../definitions"),
    )
    .expect("definitions");
    let agent = reg.get("repo-keeper").expect("repo-keeper");
    let prompt = assemble_prompt_full(Some(agent), &[], None, Some(&session), "summarize status");
    assert!(prompt.contains("Multi-agent session orchestration"));
    assert!(prompt.contains("recent_tool_results"));
    assert!(prompt.contains("git.status"));
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn live_llm_repo_keeper_reasons_with_session_context() {
    if !ollama_available().await {
        eprintln!("skip: Ollama not available at http://127.0.0.1:11434");
        return;
    }
    let dir = std::env::temp_dir().join(format!("rmng-live-llm-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let mut session = store.create().expect("create");
    store
        .record_tool_result(
            &mut session,
            ToolResultRecord {
                timestamp: chrono::Utc::now(),
                tool: "git.status".into(),
                parameters: serde_json::json!({}),
                output: "On branch main\nnothing to commit".into(),
                success: true,
                exit_code: Some(0),
                handoff_from: None,
            },
        )
        .expect("record");
    let router = test_router_with_ollama(store);
    let outcome = router
        .ask_routed(
            Some(&session.id),
            "repo-keeper",
            "The git status is already in recent_tool_results. Emit plan.only summarizing it. Do not call git.status again.",
        )
        .await
        .expect("ask");
    let intent = outcome.intent();
    assert!(
        matches!(intent, rmng_core::CoreIntent::PlanOnly { .. })
            || matches!(intent, rmng_core::CoreIntent::ToolExecute { .. }),
        "expected valid core intent from live LLM"
    );
    if let rmng_core::CoreIntent::PlanOnly { metadata, .. } = &intent {
        assert_eq!(
            metadata.as_ref().and_then(|m| m.session_id.as_deref()),
            Some(session.id.as_str())
        );
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn live_llm_swarm_coordinator_delegates_git_workflow() {
    if !ollama_available().await {
        eprintln!("skip: Ollama not available");
        return;
    }
    let dir = std::env::temp_dir().join(format!("rmng-live-l4-{}", uuid::Uuid::new_v4()));
    let store = SessionStore::new(&dir);
    let session = store.create().expect("create");
    let router = test_router_with_ollama(store);
    let outcome = router
        .ask_routed(
            Some(&session.id),
            "swarm-coordinator",
            "check git status for RMNG-OS",
        )
        .await
        .expect("orchestrate");
    assert!(
        outcome.is_handoff() || matches!(outcome, RouteOutcome::Direct { .. }),
        "L4 should produce handoff or direct intent"
    );
    let _ = std::fs::remove_dir_all(dir);
}